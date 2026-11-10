use stratum_core::config::LibraryConfig;
use stratum_core::services::LibraryService;
use stratum_core::structs::library::Library;

pub struct ServiceFactory;

impl ServiceFactory {
    pub fn with_capacity(max_structures: usize, max_projects: usize) -> LibraryService {
        LibraryService::new(Library::new(), LibraryConfig { max_structures, max_projects })
    }
}
