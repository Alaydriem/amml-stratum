use crate::structs::preview::{BlockTexture, OrbitCamera, PreviewMesh, PreviewProjection, PreviewStop};

// Draws a preview mesh into an RGBA image on the CPU: perspective correct texture lookup, nearest texel, a depth
// buffer, and texels under half alpha left out (the game's alpha test for leaves and plants). The colour and depth
// buffers are kept between renders and only grow when the image grows; `swap_pixels` hands the colour buffer out
// and takes a spare in its place, so a picture leaves without a copy. The stop is checked every `STOP_EVERY` quads.
// Bound: the buffers hold one image of at most `MAX_WIDTH * MAX_HEIGHT` pixels.
pub struct PreviewRasterizer {
    width: usize,
    height: usize,
    colour: Vec<u8>,
    // 1 / depth per pixel; 0 is empty, larger is nearer.
    depth: Vec<f32>,
    stop: PreviewStop,
}

impl PreviewRasterizer {
    pub const MAX_WIDTH: u32 = 640;
    pub const MAX_HEIGHT: u32 = 480;
    const ALPHA_CUT: u8 = 128;
    const STOP_EVERY: usize = 256;

    pub fn new(stop: PreviewStop) -> Self {
        Self { width: 0, height: 0, colour: Vec::new(), depth: Vec::new(), stop }
    }

    // Draws `mesh` seen through `camera` at `width` by `height` pixels, each capped at its limit, and returns the
    // triangles drawn. The picture is in `pixels`; empty pixels are transparent. A set stop ends the render early
    // with the picture part drawn.
    pub fn render(&mut self, mesh: &PreviewMesh, camera: OrbitCamera, width: u32, height: u32) -> usize {
        let width = width.clamp(1, Self::MAX_WIDTH);
        let height = height.clamp(1, Self::MAX_HEIGHT);
        self.width = width as usize;
        self.height = height as usize;
        let pixels = self.width * self.height;
        self.colour.clear();
        self.colour.resize(pixels * 4, 0);
        self.depth.clear();
        self.depth.resize(pixels, 0.0);
        let projection = PreviewProjection::new(mesh.view_bounds(), camera, width, height);
        let eye = projection.eye();
        let mut drawn = 0;
        for (index, quad) in mesh.quads().iter().enumerate() {
            if index % Self::STOP_EVERY == 0 && self.stop.is_stopped() {
                break;
            }
            let world = quad.corners.map(|corner| corner.map(|axis| f32::from(axis) / 16.0));
            if quad.normal != [0, 0, 0] {
                let [nx, ny, nz] = quad.normal.map(f32::from);
                let toward = (eye[0] - world[0][0]) * nx + (eye[1] - world[0][1]) * ny + (eye[2] - world[0][2]) * nz;
                if toward <= 0.0 {
                    continue;
                }
            }
            let projected = world.map(|point| projection.project(point));
            if projected.iter().any(|point| point[2] <= 0.0) {
                continue;
            }
            let vertices: [[f32; 5]; 4] = std::array::from_fn(|corner| {
                let [x, y, inverse] = projected[corner];
                let [u, v] = quad.uvs[corner].map(|axis| f32::from(axis) / 16.0);
                [x, y, inverse, u * inverse, v * inverse]
            });
            let Some(texture) = mesh.textures.get(usize::from(quad.texture)).or_else(|| mesh.textures.first()) else {
                continue;
            };
            let shade = u32::from(quad.shade);
            drawn += usize::from(self.triangle(vertices[0], vertices[1], vertices[2], texture, shade));
            drawn += usize::from(self.triangle(vertices[0], vertices[2], vertices[3], texture, shade));
        }
        drawn
    }

    pub fn width(&self) -> u32 {
        self.width as u32
    }

    pub fn height(&self) -> u32 {
        self.height as u32
    }

    // The last picture: straight RGBA rows from the top.
    pub fn pixels(&self) -> &[u8] {
        &self.colour
    }

    // Gives out the last picture and keeps `spare` as the colour buffer of the next render, which reuses its
    // capacity. A spare of the size of the picture makes the next render allocate nothing.
    pub fn swap_pixels(&mut self, spare: Vec<u8>) -> Vec<u8> {
        std::mem::replace(&mut self.colour, spare)
    }

    // Each vertex is screen x, screen y, 1 / depth, u / depth and v / depth. True when the triangle covered the
    // screen at all.
    #[inline]
    fn triangle(&mut self, a: [f32; 5], b: [f32; 5], c: [f32; 5], texture: &BlockTexture, shade: u32) -> bool {
        let area = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
        if area.abs() < 1e-6 {
            return false;
        }
        let (b, c, area) = if area < 0.0 { (c, b, -area) } else { (b, c, area) };
        let max_x = (self.width - 1) as f32;
        let max_y = (self.height - 1) as f32;
        let left = a[0].min(b[0]).min(c[0]).floor().max(0.0);
        let right = a[0].max(b[0]).max(c[0]).ceil().min(max_x);
        let top = a[1].min(b[1]).min(c[1]).floor().max(0.0);
        let bottom = a[1].max(b[1]).max(c[1]).ceil().min(max_y);
        if left > right || top > bottom {
            return false;
        }
        let inverse_area = 1.0 / area;
        // edge functions: w0 is the weight of a (edge b to c), w1 of b (edge c to a), w2 of c (edge a to b)
        let step0 = [-(c[1] - b[1]), c[0] - b[0]];
        let step1 = [-(a[1] - c[1]), a[0] - c[0]];
        let step2 = [-(b[1] - a[1]), b[0] - a[0]];
        let start_x = left + 0.5;
        let start_y = top + 0.5;
        let edge = |from: [f32; 5], to: [f32; 5]| -> f32 {
            (to[0] - from[0]) * (start_y - from[1]) - (to[1] - from[1]) * (start_x - from[0])
        };
        let mut row0 = edge(b, c);
        let mut row1 = edge(c, a);
        let mut row2 = edge(a, b);
        let (x0, x1) = (left as usize, right as usize);
        for y in top as usize..=bottom as usize {
            let (mut w0, mut w1, mut w2) = (row0, row1, row2);
            let line = y * self.width;
            for x in x0..=x1 {
                if w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0 {
                    let (l0, l1, l2) = (w0 * inverse_area, w1 * inverse_area, w2 * inverse_area);
                    let inverse = l0 * a[2] + l1 * b[2] + l2 * c[2];
                    let at = line + x;
                    if let Some(depth) = self.depth.get_mut(at)
                        && inverse > *depth
                    {
                        let z = 1.0 / inverse;
                        let u = (l0 * a[3] + l1 * b[3] + l2 * c[3]) * z;
                        let v = (l0 * a[4] + l1 * b[4] + l2 * c[4]) * z;
                        let [r, g, bl, alpha] = texture.sample(u, v);
                        if alpha >= Self::ALPHA_CUT {
                            *depth = inverse;
                            if let Some(out) = self.colour.get_mut(at * 4..at * 4 + 4) {
                                out[0] = ((u32::from(r) * shade) >> 8) as u8;
                                out[1] = ((u32::from(g) * shade) >> 8) as u8;
                                out[2] = ((u32::from(bl) * shade) >> 8) as u8;
                                out[3] = 255;
                            }
                        }
                    }
                }
                w0 += step0[0];
                w1 += step1[0];
                w2 += step2[0];
            }
            row0 += step0[1];
            row1 += step1[1];
            row2 += step2[1];
        }
        true
    }
}
