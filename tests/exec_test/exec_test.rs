use felico::interpreter::interpreter::run_program_to_string;

#[path = "../transform_test.rs"]
pub mod transform_test;

#[cfg(test)]
pub fn main() {
    transform_test::run_transform_test("tests/exec_test/testcases", |name: &str, input: &str| {
        run_program_to_string(name, input)
    });
}
