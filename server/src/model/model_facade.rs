use felico_compiler::frontend::resolve::module_manifest::ModuleManifest;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct ModelFacade {
    inner: Arc<ModelFacadeInner>,
}

impl ModelFacade {
    pub fn new(modules: Vec<ModuleManifest>) -> Self {
        Self {
            inner: Arc::new(ModelFacadeInner { modules }),
        }
    }

    pub fn modules(&self) -> &[ModuleManifest] {
        &self.inner.modules
    }
}

#[derive(Debug)]
struct ModelFacadeInner {
    modules: Vec<ModuleManifest>,
}
