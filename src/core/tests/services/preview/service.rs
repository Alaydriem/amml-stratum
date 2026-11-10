use std::time::{Duration, Instant};

use stratum_core::config::PreviewConfig;
use stratum_core::errors::PreviewError;
use stratum_core::services::preview::PreviewService;
use stratum_core::structs::library::StructureId;
use stratum_core::structs::preview::{OrbitCamera, PreviewFrame, PreviewRequest, PreviewSource};

use crate::support::{PreviewFixture, StructureFixture};

// Requests and waits for the service tests.
struct Requests;

impl Requests {
    fn plain(serial: u64, file_name: &str) -> PreviewRequest {
        PreviewRequest {
            serial,
            structure: StructureId(serial),
            file_name: file_name.to_owned(),
            camera: OrbitCamera::start(),
            width: 32,
            height: 24,
        }
    }

    // A request for the structure of serial 1, turned by `serial` degrees, so each one is a new picture.
    fn turned(serial: u64, file_name: &str) -> PreviewRequest {
        PreviewRequest {
            structure: StructureId(1),
            camera: OrbitCamera::start().turned(serial as f32, 0.0),
            ..Self::plain(serial, file_name)
        }
    }

    fn service(dir: &std::path::Path) -> PreviewService {
        PreviewService::new(PreviewSource::new(None, dir.to_path_buf()), &PreviewConfig::default())
    }

    fn wait(service: &PreviewService) -> PreviewFrame {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(frame) = service.take() {
                return frame;
            }
            assert!(Instant::now() < deadline, "no preview frame in time");
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

#[test]
fn a_request_comes_back_as_a_picture_of_the_asked_size() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = StructureFixture::new([2, 2, 2]).block(0, 0, 0, "minecraft:stone").bytes();
    std::fs::write(dir.path().join("one.mcstructure"), bytes).unwrap();
    let service = Requests::service(dir.path());
    service.submit(Requests::plain(1, "one.mcstructure")).unwrap();
    let frame = Requests::wait(&service);
    assert_eq!(frame.serial, 1);
    let image = frame.outcome.unwrap();
    assert_eq!((image.width, image.height), (32, 24));
    assert_eq!(image.rgba.len(), 32 * 24 * 4);
    assert!(image.rgba.as_chunks::<4>().0.iter().any(|texel| texel[3] == 255));
}

#[test]
fn a_missing_file_comes_back_as_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let service = Requests::service(dir.path());
    service.submit(Requests::plain(1, "gone.mcstructure")).unwrap();
    let frame = Requests::wait(&service);
    assert!(matches!(frame.outcome, Err(PreviewError::Structure(_))));
}

#[test]
fn a_failed_structure_is_not_read_again_until_the_structure_changes() {
    let dir = tempfile::tempdir().unwrap();
    let service = Requests::service(dir.path());
    service.submit(Requests::turned(1, "one.mcstructure")).unwrap();
    assert!(Requests::wait(&service).outcome.is_err());

    // the file appears; a read for the same structure would now succeed
    let bytes = StructureFixture::new([2, 2, 2]).block(0, 0, 0, "minecraft:stone").bytes();
    std::fs::write(dir.path().join("one.mcstructure"), bytes).unwrap();
    service.submit(Requests::turned(2, "one.mcstructure")).unwrap();
    let again = Requests::wait(&service);
    assert!(matches!(again.outcome, Err(PreviewError::Structure(_))), "the failed structure was read again");

    service.submit(Requests::plain(3, "one.mcstructure")).unwrap();
    assert!(Requests::wait(&service).outcome.is_ok(), "another structure is read");
}

#[test]
fn pictures_given_back_are_drawn_into_again() {
    let dir = tempfile::tempdir().unwrap();
    let bytes = StructureFixture::new([2, 2, 2]).block(0, 0, 0, "minecraft:stone").bytes();
    std::fs::write(dir.path().join("one.mcstructure"), bytes).unwrap();
    let service = Requests::service(dir.path());
    let mut buffers = Vec::new();
    for serial in 1..=4 {
        service.submit(Requests::turned(serial, "one.mcstructure")).unwrap();
        let image = Requests::wait(&service).outcome.unwrap();
        buffers.push(image.rgba.as_ptr() as usize);
        service.give_back(image.rgba);
    }
    buffers.sort_unstable();
    buffers.dedup();
    assert_eq!(buffers.len(), 2, "four pictures used {} buffers", buffers.len());
}

#[test]
fn an_idle_service_stops_at_once_and_then_refuses_requests() {
    let dir = tempfile::tempdir().unwrap();
    let service = Requests::service(dir.path());
    std::thread::sleep(Duration::from_millis(20));
    let started = Instant::now();
    assert!(service.shutdown(Duration::from_secs(5)));
    assert!(started.elapsed() < Duration::from_millis(50), "shutdown took {:?}", started.elapsed());
    assert!(service.submit(Requests::plain(1, "one.mcstructure")).is_err());
    assert!(service.shutdown(Duration::ZERO), "a second shutdown has nothing to wait for");
}

#[test]
fn shutdown_during_a_long_mesh_build_returns_within_the_budget() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("big.mcstructure"), StructureFixture::full([192, 192, 192]).bytes()).unwrap();
    let service = Requests::service(dir.path());
    service.submit(Requests::plain(1, "big.mcstructure")).unwrap();
    // the read, parse and mesh build take about 2.5 s on the test profile; the parse and the build check the
    // stop, so the shutdown can land in either; 50 ms only lets the thread take the request
    std::thread::sleep(Duration::from_millis(50));
    let budget = Duration::from_millis(600);
    let started = Instant::now();
    assert!(service.shutdown(budget), "the thread did not end within the budget");
    assert!(started.elapsed() < budget);
    assert!(service.take().is_none(), "a stopped build sends no picture");
}

#[test]
fn a_reconfigured_service_evicts_textures_by_the_new_bound() {
    let packs = tempfile::tempdir().unwrap();
    let names = ["red", "green", "blue", "gold"];
    let files: Vec<(String, Vec<u8>)> = names
        .iter()
        .enumerate()
        .map(|(index, name)| (format!("{name}.tga"), PreviewFixture::tga(1, &[vec![[index as u8 * 60, 9, 9, 255]]])))
        .collect();
    let files: Vec<(&str, Vec<u8>)> = files.iter().map(|(name, bytes)| (name.as_str(), bytes.clone())).collect();
    PreviewFixture::pack(packs.path(), "{}", r#"{ "texture_data": {} }"#, &files);
    let structures = tempfile::tempdir().unwrap();
    let mut fixture = StructureFixture::new([7, 1, 1]);
    for (index, name) in names.iter().enumerate() {
        fixture = fixture.block(index * 2, 0, 0, &format!("minecraft:{name}"));
    }
    std::fs::write(structures.path().join("four.mcstructure"), fixture.bytes()).unwrap();
    let source = PreviewSource::new(Some(packs.path().to_path_buf()), structures.path().to_path_buf());
    let service = PreviewService::new(source, &PreviewConfig::default());

    service.submit(Requests::plain(1, "four.mcstructure")).unwrap();
    let wide = Requests::wait(&service).outcome.unwrap();
    assert_eq!(wide.stats.textures_cached, 4);

    service.reconfigure(&PreviewConfig { texture_cache: 2, ..PreviewConfig::default() });
    service.submit(Requests::plain(2, "four.mcstructure")).unwrap();
    let narrow = Requests::wait(&service).outcome.unwrap();
    assert!(
        (1..=2).contains(&narrow.stats.textures_cached),
        "the cache holds {} textures under a bound of 2",
        narrow.stats.textures_cached
    );
}

#[test]
fn a_shutdown_after_one_that_ran_out_of_budget_reports_the_real_state() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("big.mcstructure"), StructureFixture::full([192, 192, 192]).bytes()).unwrap();
    let service = Requests::service(dir.path());
    service.submit(Requests::plain(1, "big.mcstructure")).unwrap();
    std::thread::sleep(Duration::from_millis(50));
    // no budget: the thread cannot end in no time, so both calls find it still running
    assert!(!service.shutdown(Duration::ZERO));
    assert!(!service.shutdown(Duration::ZERO), "a second call reported an end the thread had not reached");
    assert!(service.shutdown(Duration::from_secs(2)));
}
