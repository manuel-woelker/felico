use felico_compiler::frontend::resolve::module_manifest::BundleManifest;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct ModelFacade<'ws> {
    inner: Arc<ModelFacadeInner<'ws>>,
}

impl<'ws> ModelFacade<'ws> {
    pub fn new(bundles: Vec<BundleManifest<'ws>>) -> Self {
        Self {
            inner: Arc::new(ModelFacadeInner { bundles }),
        }
    }

    pub fn bundles(&self) -> &[BundleManifest] {
        &self.inner.bundles
    }
}

#[derive(Debug)]
struct ModelFacadeInner<'ws> {
    bundles: Vec<BundleManifest<'ws>>,
}
