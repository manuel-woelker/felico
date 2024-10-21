use felico_compiler::backend::c::c_code_generator::CCodeGenerator;
use felico_compiler::backend::c::c_compiler::CCompiler;
use felico_compiler::bail;
use felico_compiler::frontend::parse::parser::Parser;
use felico_compiler::frontend::resolve::resolver::Resolver;
use felico_compiler::infra::arena::Arena;
use felico_compiler::model::workspace::Workspace;
use std::path::PathBuf;
use std::process::{Command, Stdio};

#[path = "../transform_test.rs"]
pub mod transform_test;

#[cfg(test)]
pub fn main() {
    transform_test::run_transform_test("tests/exec_test/testcases", |name: &str, input: &str| {
        let arena = Arena::new();
        let workspace = Workspace::new(&arena);
        let source_file = workspace.source_file_from_string(format!("{}.felico", name), input);
        let mut parser = Parser::new(source_file, workspace).unwrap();
        let mut ast = parser.parse_script().unwrap();
        let mut resolver = Resolver::new(workspace);
        resolver.resolve_program(&mut ast).unwrap();

        let source_path = PathBuf::from(format!("target/felico/{}.c", name));
        std::fs::write(&source_path, input).unwrap();
        let binary_path = source_path.join(format!("../{}.exe", name));
        std::fs::create_dir_all(source_path.parent().unwrap()).unwrap();
        let mut code_generator = CCodeGenerator::new(&ast, &source_path).unwrap();
        code_generator.generate_code().unwrap();
        let c_compiler = CCompiler::new().unwrap();
        c_compiler.compile(&source_path, &binary_path).unwrap();
        let mut command = Command::new(&binary_path);
        let output = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .unwrap();
        if !output.status.success() {
            bail!(
                "Exit status: {}, \n\tStdout: {} \n\tStderr: {}",
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            )
        }
        let output = String::from_utf8(output.stdout).unwrap();
        Ok(output)
    });
}
