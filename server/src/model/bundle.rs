use serde::Serialize;

#[derive(Serialize)]
pub struct BundleInfo {
    pub name: String,
    pub version: String,
}

#[derive(Serialize)]
pub struct BundleDescription {
    pub info: BundleInfo,
    pub functions: Vec<FunctionDescription>,
}

#[derive(Serialize)]
pub struct FunctionDescription {
    pub name: String,
    pub signature: String,
}

#[derive(Serialize)]
pub struct BundleIndex {
    pub bundles: Vec<BundleInfo>,
}
