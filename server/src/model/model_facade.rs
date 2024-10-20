use crate::model::bundle::{BundleDescription, BundleInfo, FunctionDescription, ModuleDescription};
use felico_compiler::frontend::resolve::module_manifest::BundleManifest;
use std::sync::Arc;

#[derive(Clone)]
pub struct ModelFacade {
    inner: Arc<ModelFacadeInner>,
}

impl ModelFacade {
    pub fn new(bundles: Vec<BundleManifest>) -> Self {
        let bundles_descriptions = bundles
            .iter()
            .map(|bundle| BundleDescription {
                info: BundleInfo {
                    name: bundle.name.to_string(),
                    version: "TODO: Version".to_string(),
                    /*
                    functions: vec![FunctionDescription {
                        name: "debug_print".to_string(),
                        signature: "fun debug_print(item: any)".to_string(),
                    }],*/
                },
                modules: bundle
                    .modules
                    .iter()
                    .map(|module| ModuleDescription {
                        name: module.name.to_string(),
                        functions: module
                            .module_entries
                            .values()
                            .map(|entry| FunctionDescription {
                                name: entry.name.to_string(),
                                signature: entry.type_signature.clone(),
                            })
                            .collect(),
                    })
                    .collect(),
                /*        functions: vec![FunctionDescription {
                    name: "debug_print".to_string(),
                    signature: "fun debug_print(item: any)".to_string(),
                }],*/
            })
            .collect();
        Self {
            inner: Arc::new(ModelFacadeInner {
                bundles: bundles_descriptions,
            }),
        }
    }

    pub fn bundles(&self) -> &[BundleDescription] {
        &self.inner.bundles
    }
}

//#[derive(Debug)]
struct ModelFacadeInner {
    bundles: Vec<BundleDescription>,
}
