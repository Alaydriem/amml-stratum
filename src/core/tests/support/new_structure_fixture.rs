use stratum_core::structs::library::NewStructure;
use stratum_core::structs::structure::StructureSize;

pub struct NewStructureFixture;

impl NewStructureFixture {
    pub fn named(name: &str) -> NewStructure {
        NewStructure {
            display_name: name.to_string(),
            file_name: format!("{name}.mcstructure"),
            size: StructureSize::new(4, 5, 6),
            block_count: 120,
            world_origin: None,
            project: None,
        }
    }
}
