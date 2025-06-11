use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

pub trait ModuleProvider {
    fn get_module(&self, path: &Path) -> Option<&str>;
}

#[derive(Default, Debug)]
pub struct MemoryModules {
    pub modules: HashMap<PathBuf, String>,
}

impl ModuleProvider for MemoryModules {
    fn get_module(&self, path: &Path) -> Option<&str> {
        self.modules.get(path).map(|s| s.as_str())
    }
}
