use stratum_core::services::LibraryService;
use stratum_core::structs::library::{ProjectId, StructureId};

use crate::support::{NewStructureFixture, ServiceFactory};

struct Fixture;

impl Fixture {
    // A service with one project holding the named structures, in the order given.
    fn project_with(names: &[&str]) -> (LibraryService, ProjectId, Vec<StructureId>) {
        let mut svc = ServiceFactory::with_capacity(10, 10);
        let project = svc.create_project("P").unwrap();
        let mut ids = Vec::new();
        for name in names {
            let mut new = NewStructureFixture::named(name);
            new.project = Some(project);
            ids.push(svc.add_structure(new).unwrap());
        }
        (svc, project, ids)
    }
}

#[test]
fn orders_by_name_ignoring_case_then_id() {
    let (svc, project, ids) = Fixture::project_with(&["b", "A", "a"]);
    assert_eq!(svc.order().of(project), [ids[1], ids[2], ids[0]]);
    assert_eq!(svc.order().position(project, ids[0]), Some(2));
}

#[test]
fn step_wraps_both_ways() {
    let (svc, project, ids) = Fixture::project_with(&["a", "b", "c"]);
    let order = svc.order();
    assert_eq!(order.step(project, ids[0], true), Some(ids[1]));
    assert_eq!(order.step(project, ids[2], true), Some(ids[0]));
    assert_eq!(order.step(project, ids[0], false), Some(ids[2]));
    assert_eq!(order.step(project, ids[1], false), Some(ids[0]));
}

#[test]
fn step_on_a_single_structure_returns_it() {
    let (svc, project, ids) = Fixture::project_with(&["a"]);
    assert_eq!(svc.order().step(project, ids[0], true), Some(ids[0]));
    assert_eq!(svc.order().step(project, ids[0], false), Some(ids[0]));
}

#[test]
fn step_from_a_structure_outside_the_project_is_none() {
    let (mut svc, project, _) = Fixture::project_with(&["a", "b"]);
    let outside = svc.add_structure(NewStructureFixture::named("c")).unwrap();
    assert_eq!(svc.order().step(project, outside, true), None);
    assert_eq!(svc.order().step(project, StructureId(99), false), None);
}

#[test]
fn unknown_project_is_empty() {
    let (svc, _, ids) = Fixture::project_with(&["a"]);
    assert!(svc.order().of(ProjectId(99)).is_empty());
    assert_eq!(svc.order().position(ProjectId(99), ids[0]), None);
    assert_eq!(svc.order().step(ProjectId(99), ids[0], true), None);
}
