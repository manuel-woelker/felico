use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct BundleInfo {
    pub name: String,
    pub version: String,
}

#[derive(Serialize, Clone)]
pub struct BundleDescription {
    pub info: BundleInfo,
    pub modules: Vec<ModuleDescription>,
}

#[derive(Serialize, Clone)]
pub struct ModuleDescription {
    pub name: String,
    pub functions: Vec<FunctionDescription>,
}

#[derive(Serialize, Clone)]
pub struct FunctionDescription {
    pub name: String,
    pub signature: String,
}

#[derive(Serialize, Clone)]
pub struct BundleIndex {
    pub bundles: Vec<BundleInfo>,
}
