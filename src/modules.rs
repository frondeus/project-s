use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::source::{Source, SourceId, Sources};

pub trait ModuleProvider {
    fn get_module(&self, path: &Path) -> Option<SourceId>;
    fn get_source(&self, source_id: SourceId) -> Option<&Source>;
}

#[derive(Default, Debug)]
pub struct MemoryModules {
    pub modules: HashMap<PathBuf, SourceId>,
    pub sources: Sources,
}

impl ModuleProvider for MemoryModules {
    fn get_module(&self, path: &Path) -> Option<SourceId> {
        self.modules.get(path).cloned()
    }

    fn get_source(&self, source_id: SourceId) -> Option<&Source> {
        Some(self.sources.get(source_id))
    }
}
