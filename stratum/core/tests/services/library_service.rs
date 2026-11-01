use stratum_core::errors::LibraryError;
use stratum_core::structs::library::{ProjectId, ProjectKind, StructureId};

use crate::support::{NewStructureFixture, ServiceFactory};

#[test]
fn add_rename_move_delete() {
    let mut svc = ServiceFactory::with_capacity(10, 10);
    let project = svc.create_project("Castle").unwrap();
    let id = svc.add_structure(NewStructureFixture::named("Tower")).unwrap();
    assert!(svc.is_dirty());
    svc.mark_saved();
    assert!(!svc.is_dirty());

    svc.rename_structure(id, "Keep").unwrap();
    svc.move_structure(id, Some(project)).unwrap();
    assert_eq!(svc.structures_in(Some(project))[0].display_name, "Keep");
    assert!(svc.structures_in(None).is_empty());
    assert!(svc.is_dirty());

    let removed = svc.delete_structure(id).unwrap();
    assert_eq!(removed.file_name, "Tower.mcstructure");
    assert!(svc.library().structures.is_empty());
    assert_eq!(svc.delete_structure(id), Err(LibraryError::NotFound));
    assert_eq!(svc.move_structure(id, None), Err(LibraryError::NotFound));
}

#[test]
fn move_to_unknown_project_is_refused() {
    let mut svc = ServiceFactory::with_capacity(10, 10);
    let id = svc.add_structure(NewStructureFixture::named("A")).unwrap();
    assert_eq!(svc.move_structure(id, Some(ProjectId(99))), Err(LibraryError::NotFound));
}

#[test]
fn rename_changes_display_name_only() {
    let mut svc = ServiceFactory::with_capacity(10, 10);
    let id = svc.add_structure(NewStructureFixture::named("Tower")).unwrap();
    svc.rename_structure(id, "  Keep  ").unwrap();
    let entry = &svc.library().structures[0];
    assert_eq!(entry.display_name, "Keep");
    assert_eq!(entry.file_name, "Tower.mcstructure");
}

#[test]
fn names_are_trimmed_and_checked() {
    let mut svc = ServiceFactory::with_capacity(10, 10);
    assert_eq!(svc.create_project("   "), Err(LibraryError::NameEmpty));
    assert_eq!(svc.create_project(&"x".repeat(65)), Err(LibraryError::NameTooLong));
    let ok = svc.create_project(&format!(" {} ", "y".repeat(64))).unwrap();
    assert_eq!(svc.library().projects[0].id, ok);
    assert_eq!(svc.library().projects[0].name.chars().count(), 64);

    assert_eq!(svc.add_structure(NewStructureFixture::named("")), Err(LibraryError::NameEmpty));
    let id = svc.add_structure(NewStructureFixture::named("A")).unwrap();
    assert_eq!(svc.rename_structure(id, &"z".repeat(65)), Err(LibraryError::NameTooLong));
    assert_eq!(svc.rename_project(ok, ""), Err(LibraryError::NameEmpty));
    assert_eq!(svc.library().structures.len(), 1);
}

#[test]
fn refuses_structures_past_capacity() {
    let mut svc = ServiceFactory::with_capacity(3, 10);
    for name in ["a", "b", "c"] {
        svc.add_structure(NewStructureFixture::named(name)).unwrap();
    }
    assert_eq!(svc.add_structure(NewStructureFixture::named("d")), Err(LibraryError::Full { capacity: 3 }));
    assert_eq!(svc.library().structures.len(), 3);
}

#[test]
fn refuses_projects_past_capacity() {
    let mut svc = ServiceFactory::with_capacity(10, 2);
    svc.create_project("a").unwrap();
    svc.create_project("b").unwrap();
    assert_eq!(svc.create_project("c"), Err(LibraryError::Full { capacity: 2 }));
    assert_eq!(svc.library().projects.len(), 2);
}

#[test]
fn deleting_a_project_moves_its_structures_to_unsorted() {
    let mut svc = ServiceFactory::with_capacity(10, 10);
    let project = svc.create_project("Castle").unwrap();
    let mut new = NewStructureFixture::named("Tower");
    new.project = Some(project);
    svc.add_structure(new).unwrap();
    svc.set_project_kind(project, ProjectKind::BigBuild).unwrap();
    assert_eq!(svc.library().projects[0].kind, ProjectKind::BigBuild);

    svc.delete_project(project).unwrap();
    assert!(svc.library().projects.is_empty());
    assert_eq!(svc.structures_in(None).len(), 1);
    assert_eq!(svc.delete_project(project), Err(LibraryError::NotFound));
}

#[test]
fn adding_to_unknown_project_is_refused() {
    let mut svc = ServiceFactory::with_capacity(10, 10);
    let mut new = NewStructureFixture::named("Tower");
    new.project = Some(ProjectId(7));
    assert_eq!(svc.add_structure(new), Err(LibraryError::NotFound));
}

#[test]
fn ids_are_never_reused() {
    let mut svc = ServiceFactory::with_capacity(10, 10);
    for name in ["a", "b", "c"] {
        svc.add_structure(NewStructureFixture::named(name)).unwrap();
    }
    svc.delete_structure(StructureId(2)).unwrap();
    let next = svc.add_structure(NewStructureFixture::named("d")).unwrap();
    assert_eq!(next, StructureId(4));

    let p1 = svc.create_project("p1").unwrap();
    svc.delete_project(p1).unwrap();
    assert_ne!(svc.create_project("p2").unwrap(), p1);
}

#[test]
fn structures_sort_case_insensitive() {
    let mut svc = ServiceFactory::with_capacity(10, 10);
    for name in ["banana", "Apple", "cherry", "Avocado"] {
        svc.add_structure(NewStructureFixture::named(name)).unwrap();
    }
    let names: Vec<&str> = svc.structures_in(None).iter().map(|e| e.display_name.as_str()).collect();
    assert_eq!(names, ["Apple", "Avocado", "banana", "cherry"]);
}

#[test]
fn duplicate_names_get_a_suffix() {
    let mut svc = ServiceFactory::with_capacity(10, 10);
    let project = svc.create_project("Castle").unwrap();
    for _ in 0..3 {
        svc.add_structure(NewStructureFixture::named("Tower")).unwrap();
    }
    let mut other = NewStructureFixture::named("Tower");
    other.project = Some(project);
    svc.add_structure(other).unwrap();

    let names: Vec<&str> = svc.structures_in(None).iter().map(|e| e.display_name.as_str()).collect();
    assert_eq!(names, ["Tower", "Tower (2)", "Tower (3)"]);
    assert_eq!(svc.structures_in(Some(project))[0].display_name, "Tower");
}

#[test]
fn suffixed_names_stay_within_the_name_limit() {
    let mut svc = ServiceFactory::with_capacity(10, 10);
    let long = "w".repeat(64);
    svc.add_structure(NewStructureFixture::named(&long)).unwrap();
    svc.add_structure(NewStructureFixture::named(&long)).unwrap();
    let second = svc.structures_in(None).into_iter().find(|e| e.display_name.ends_with("(2)")).unwrap();
    assert_eq!(second.display_name.chars().count(), 64);
}

#[test]
fn refuses_file_names_that_are_not_plain() {
    let mut svc = ServiceFactory::with_capacity(10, 10);
    for bad in ["", ".", "..", "a/b.mcstructure", "a\\b.mcstructure", "C:evil.mcstructure"] {
        let mut new = NewStructureFixture::named("A");
        new.file_name = bad.to_string();
        assert_eq!(svc.add_structure(new), Err(LibraryError::BadFileName), "{bad}");
    }
    assert!(svc.library().structures.is_empty());
}

#[test]
fn grid_cell_is_set_and_cleared() {
    use stratum_core::structs::layout::GridCell;
    let mut svc = ServiceFactory::with_capacity(10, 10);
    let id = svc.add_structure(NewStructureFixture::named("A")).unwrap();
    svc.mark_saved();
    svc.set_grid_cell(id, Some(GridCell::new(2, 3))).unwrap();
    assert_eq!(svc.library().structures[0].grid_cell, Some(GridCell::new(2, 3)));
    assert!(svc.is_dirty());
    svc.set_grid_cell(id, None).unwrap();
    assert_eq!(svc.library().structures[0].grid_cell, None);
    assert_eq!(svc.set_grid_cell(StructureId(99), None), Err(LibraryError::NotFound));
}
