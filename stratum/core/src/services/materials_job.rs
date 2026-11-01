use std::path::PathBuf;

use crate::structs::action::MaterialScope;
use crate::structs::library::StructureId;
use crate::structs::material::MaterialList;

// A material list being built. The controller feeds the worker one missing structure at a time,
// so at most one load of the job is outstanding.
pub(crate) struct MaterialsJob {
    pub(crate) scope: MaterialScope,
    // The structures of the scope, in order.
    pub(crate) wanted: Vec<(StructureId, PathBuf)>,
    // How many of `wanted` are counted into `lists`.
    pub(crate) done: usize,
    pub(crate) lists: Vec<MaterialList>,
    // The structure the worker is loading for this job.
    pub(crate) awaiting: Option<StructureId>,
}
