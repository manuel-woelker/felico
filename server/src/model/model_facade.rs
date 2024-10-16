use felico_compiler::frontend::resolve::module_manifest::BundleManifest;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct ModelFacade {
    inner: Arc<ModelFacadeInner>,
}

impl ModelFacade {
    pub fn new(bundles: Vec<BundleManifest>) -> Self {
        Self {
            inner: Arc::new(ModelFacadeInner { bundles }),
        }
    }

    pub fn bundles(&self) -> &[BundleManifest] {
        &self.inner.bundles
    }
}

#[derive(Debug)]
struct ModelFacadeInner {
    bundles: Vec<BundleManifest>,
}
