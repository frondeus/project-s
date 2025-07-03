use std::{
    collections::{HashMap, hash_map::Entry},
    path::{Path, PathBuf},
};

use crate::source::{Source, SourceId, Sources};

pub trait ModuleProvider: 'static + Send + Sync {
    fn sources_mut(&mut self) -> &mut Sources;
    fn sources(&self) -> &Sources;

    fn get_module(&mut self, path: &Path) -> Option<SourceId>;
    fn get_source(&self, source_id: SourceId) -> Option<&Source>;

    fn get_source_from_path(&mut self, path: &Path) -> Option<&Source> {
        self.get_module(path).and_then(|id| self.get_source(id))
    }
}

#[derive(Default, Debug)]
pub struct MemoryModules {
    pub modules: HashMap<PathBuf, SourceId>,
    pub sources: Sources,
}

impl ModuleProvider for MemoryModules {
    fn get_module(&mut self, path: &Path) -> Option<SourceId> {
        self.modules.get(path).cloned()
    }

    fn get_source(&self, source_id: SourceId) -> Option<&Source> {
        Some(self.sources.get(source_id))
    }

    fn sources_mut(&mut self) -> &mut Sources {
        &mut self.sources
    }

    fn sources(&self) -> &Sources {
        &self.sources
    }
}

pub struct FileModules {
    sources: Sources,
    path_to_id: HashMap<PathBuf, SourceId>,
}
impl From<Sources> for FileModules {
    fn from(sources: Sources) -> Self {
        let path_to_id = sources
            .iter_with_id()
            .map(|(id, src)| {
                let path = PathBuf::from(&*src.filename);
                (path, id)
            })
            .collect();
        Self {
            sources,
            path_to_id,
        }
    }
}
impl ModuleProvider for FileModules {
    fn sources_mut(&mut self) -> &mut Sources {
        &mut self.sources
    }

    fn sources(&self) -> &Sources {
        &self.sources
    }

    fn get_module(&mut self, path: &std::path::Path) -> Option<SourceId> {
        let pathb = path.to_path_buf();
        match self.path_to_id.entry(pathb) {
            Entry::Occupied(entry) => Some(*entry.get()),
            Entry::Vacant(entry) => {
                let source = std::fs::read_to_string(path).ok()?;
                let path = path.display().to_string();
                let source_id = self.sources.add(&path, &source);
                entry.insert(source_id);
                Some(source_id)
            }
        }
    }

    fn get_source(&self, source_id: SourceId) -> Option<&Source> {
        Some(self.sources.get(source_id))
    }
}

#[cfg(test)]
impl MemoryModules {
    pub fn from_deps(
        input: &str,
        deps: &HashMap<test_runner::CowStr<'_>, &str>,
    ) -> (Self, SourceId) {
        let mut sources: Sources = Default::default();
        let input_id = sources.add("<input>", input);
        let modules = deps
            .iter()
            .filter(|(name, _value)| name.ends_with(".s"))
            .map(|(name, value)| {
                (
                    PathBuf::from(name.to_string()),
                    sources.add(name.to_string().as_str(), value),
                )
            })
            .collect::<HashMap<_, _>>();
        // tracing::info!("Modules: {:#?}", modules);
        (Self { modules, sources }, input_id)
    }
}
