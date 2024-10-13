use serde::Serialize;

#[derive(Serialize)]
struct PackageInfo {
    name: String,
    version: String,
}

#[derive(Serialize)]
struct PackageDescription {
    info: PackageInfo,
    functions: Vec<FunctionDescription>,
}

#[derive(Serialize)]
struct FunctionDescription {
    name: String,
    signature: String,
}

#[derive(Serialize)]
struct PackageIndex {
    packages: Vec<PackageInfo>,
}
