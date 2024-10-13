use serde::Serialize;

#[derive(Serialize)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
}

#[derive(Serialize)]
pub struct PackageDescription {
    pub info: PackageInfo,
    pub functions: Vec<FunctionDescription>,
}

#[derive(Serialize)]
pub struct FunctionDescription {
    pub name: String,
    pub signature: String,
}

#[derive(Serialize)]
pub struct PackageIndex {
    pub packages: Vec<PackageInfo>,
}
