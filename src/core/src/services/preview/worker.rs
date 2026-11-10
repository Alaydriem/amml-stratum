use std::time::{Duration, Instant};

use crossbeam_channel::Receiver;

use super::{PreviewMesher, PreviewRasterizer, TexturePack};
use crate::config::PreviewConfig;
use crate::errors::{PreviewError, StructureError};
use crate::services::StructureParser;
use crate::structs::library::StructureId;
use crate::structs::preview::{
    PreviewFrame, PreviewImage, PreviewMesh, PreviewRequest, PreviewSource, PreviewStats, PreviewStop,
};

// The state of the preview thread: the texture pack, read on the first request, the mesh of the last structure,
// the rasteriser's buffers and a spare pixel buffer for the next picture.
// Bound: one mesh is kept; a request for another structure replaces it. One spare buffer is kept; the UI gives
// buffers back through `spares`, which the service bounds.
pub(crate) struct PreviewWorker {
    source: PreviewSource,
    texture_cache: usize,
    stop: PreviewStop,
    pack: Option<TexturePack>,
    mesh: Option<(StructureId, PreviewMesh)>,
    // The last structure and file that failed to load, with the error. Requests for the same pair are answered
    // from it without a read; another structure or file, or a new mesh cap, drops it.
    failed: Option<(StructureId, String, PreviewError)>,
    mesher: PreviewMesher,
    rasterizer: PreviewRasterizer,
    spares: Receiver<Vec<u8>>,
    spare: Option<Vec<u8>>,
    configs: Receiver<PreviewConfig>,
    max_quads: usize,
    last_log: Option<Instant>,
}

impl PreviewWorker {
    const LOG_EVERY: Duration = Duration::from_millis(250);

    pub(crate) fn new(
        source: PreviewSource,
        config: &PreviewConfig,
        stop: PreviewStop,
        spares: Receiver<Vec<u8>>,
        configs: Receiver<PreviewConfig>,
    ) -> Self {
        Self {
            source,
            texture_cache: config.texture_cache(),
            pack: None,
            mesh: None,
            failed: None,
            mesher: PreviewMesher::new(config.max_quads(), stop.clone()),
            rasterizer: PreviewRasterizer::new(stop.clone()),
            stop,
            spares,
            spare: None,
            configs,
            max_quads: config.max_quads(),
            last_log: None,
        }
    }

    pub(crate) fn handle(&mut self, request: &PreviewRequest) -> PreviewFrame {
        self.apply_configs();
        let outcome = self.render(request);
        if let Err(error) = &outcome
            && *error != PreviewError::Stopped
        {
            curia::warn!("preview render failed", { structure: request.structure.0, error: error.to_string() });
        }
        PreviewFrame { serial: request.serial, structure: request.structure, outcome }
    }

    // Takes the newest config the service sent, if any: the cache bound applies to the pack now, and a new mesh
    // cap drops the kept mesh so the next request builds it under the new cap.
    fn apply_configs(&mut self) {
        let Some(config) = self.configs.try_iter().last() else {
            return;
        };
        self.texture_cache = config.texture_cache();
        if let Some(pack) = &mut self.pack {
            pack.set_capacity(self.texture_cache);
        }
        if config.max_quads() != self.max_quads {
            self.max_quads = config.max_quads();
            self.mesher = PreviewMesher::new(self.max_quads, self.stop.clone());
            self.mesh = None;
            self.failed = None;
        }
    }

    // Keeps the pixel buffer of a picture nobody will see, for the next one.
    pub(crate) fn reclaim(&mut self, frame: PreviewFrame) {
        if let Ok(image) = frame.outcome
            && self.spare.is_none()
        {
            self.spare = Some(image.rgba);
        }
    }

    fn render(&mut self, request: &PreviewRequest) -> Result<PreviewImage, PreviewError> {
        let texture_cache = self.texture_cache;
        let pack = self.pack.get_or_insert_with(|| Self::load_pack(&self.source, texture_cache, &self.stop));
        if self.stop.is_stopped() {
            return Err(PreviewError::Stopped);
        }
        let mut mesh_us = 0;
        let mesh = match self.mesh.take() {
            Some((id, mesh)) if id == request.structure => mesh,
            _ => {
                if let Some((id, file_name, error)) = &self.failed
                    && *id == request.structure
                    && *file_name == request.file_name
                {
                    return Err(error.clone());
                }
                let started = Instant::now();
                let built = (|| -> Result<PreviewMesh, PreviewError> {
                    let path = self.source.structures_dir.join(&request.file_name);
                    let bytes = std::fs::read(&path).map_err(|e| StructureError::Unreadable(e.to_string()))?;
                    let structure = match StructureParser::parse_until(&bytes, || self.stop.is_stopped()) {
                        Ok(structure) => structure,
                        Err(StructureError::Stopped) => return Err(PreviewError::Stopped),
                        Err(error) => return Err(error.into()),
                    };
                    self.mesher.build(&structure, pack)
                })();
                let mesh = match built {
                    Ok(mesh) => mesh,
                    Err(error) => {
                        if error != PreviewError::Stopped {
                            self.failed = Some((request.structure, request.file_name.clone(), error.clone()));
                        }
                        return Err(error);
                    }
                };
                self.failed = None;
                mesh_us = Self::micros(started);
                curia::info!("preview mesh built", {
                    structure: request.structure.0,
                    mesh_us: mesh_us,
                    quads: mesh.quads().len(),
                    textures: mesh.textures.len(),
                    missing_blocks: mesh.missing,
                    truncated: mesh.truncated,
                });
                mesh
            }
        };
        let started = Instant::now();
        let pack_cached = pack.cached();
        let triangles = self.rasterizer.render(&mesh, request.camera, request.width, request.height);
        let render_us = Self::micros(started);
        if self.stop.is_stopped() {
            return Err(PreviewError::Stopped);
        }
        let stats = PreviewStats {
            render_us,
            mesh_us,
            quads: mesh.quads().len(),
            triangles,
            width: self.rasterizer.width(),
            height: self.rasterizer.height(),
            size: mesh.size,
            textures_cached: pack_cached,
        };
        let spare = self.spare.take().or_else(|| self.spares.try_recv().ok()).unwrap_or_default();
        let image =
            PreviewImage { width: stats.width, height: stats.height, rgba: self.rasterizer.swap_pixels(spare), stats };
        self.mesh = Some((request.structure, mesh));
        if self.last_log.is_none_or(|at| at.elapsed() >= Self::LOG_EVERY) {
            self.last_log = Some(Instant::now());
            curia::info!("preview render", {
                structure: request.structure.0,
                render_us: stats.render_us,
                triangles: stats.triangles,
                quads: stats.quads,
                width: stats.width,
                height: stats.height,
                size_x: stats.size.x,
                size_y: stats.size.y,
                size_z: stats.size.z,
                yaw: request.camera.yaw,
                pitch: request.camera.pitch,
            });
        }
        Ok(image)
    }

    fn load_pack(source: &PreviewSource, texture_cache: usize, stop: &PreviewStop) -> TexturePack {
        let started = Instant::now();
        let pack = match &source.packs_dir {
            Some(dir) => TexturePack::load_until(dir, texture_cache, || stop.is_stopped()),
            None => TexturePack::empty(texture_cache),
        };
        curia::info!("preview textures loaded", {
            load_us: Self::micros(started),
            packs: pack.pack_count(),
            blocks: pack.block_count(),
            texture_files: pack.file_count(),
            archive_bytes: pack.archive_bytes(),
            found: source.packs_dir.is_some(),
        });
        pack
    }

    fn micros(started: Instant) -> u64 {
        u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX)
    }
}
