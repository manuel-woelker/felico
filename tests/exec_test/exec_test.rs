
#[path = "../transform_test.rs"]
pub mod transform_test;

#[cfg(test)]
pub fn main() {
    transform_test::run_transform_test("tests/exec_test/testcases/simple.yaml", |input: &str| {
        input.to_string()+input
    });
}