use crate::frontend::ast::expr::{BlockExpr, CallExpr, Expr, LiteralExpr, VarUse};
use crate::frontend::ast::module::Module;
use crate::frontend::ast::node::AstNode;
use crate::frontend::ast::stmt::{FunStmt, Stmt};
use crate::infra::result::{bail, FelicoResult};
use itertools::{Itertools, Position};
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub fn generate_c_code<'a, 'ws, O: AsRef<Path>>(
    ast: &'a AstNode<'ws, Module<'ws>>,
    output_file: O,
) -> FelicoResult<()> {
    let mut code_generator = CCodeGenerator::new(ast, output_file)?;
    code_generator.generate_code()?;
    Ok(())
}

pub struct CCodeGenerator<'a, 'ws> {
    ast: &'a AstNode<'ws, Module<'ws>>,
    output_file: File,
}

impl<'a, 'ws> CCodeGenerator<'a, 'ws> {
    pub fn new<O: AsRef<Path>>(
        ast: &'a AstNode<'ws, Module<'ws>>,
        output_path: O,
    ) -> FelicoResult<Self> {
        Ok(Self {
            ast,
            output_file: File::create(output_path)?,
        })
    }

    pub fn generate_code(&mut self) -> FelicoResult<()> {
        write!(
            self.output_file,
            "{}",
            r#"
        
        #include <stdio.h>"
        
        int main(int argc, char **argv) {
            fprintf(stderr, "Hello, World!\n");
            return 123;
        }

        "#,
        )?;
        self.generate_module(self.ast)?;
        Ok(())
    }

    fn generate_module(&mut self, ast: &AstNode<'ws, Module<'ws>>) -> FelicoResult<()> {
        for stmt in &ast.data.stmts {
            self.generate_stmt(stmt)?;
        }
        Ok(())
    }

    fn generate_stmt(&mut self, stmt: &AstNode<Stmt>) -> FelicoResult<()> {
        match &*stmt.data {
            Stmt::Expression(expr_stmt) => {
                self.generate_expr(&expr_stmt.expression)?;
            }
            Stmt::Fun(fun_stmt) => {
                self.generate_fun_stmt(fun_stmt)?;
            }
            _ => {
                bail!("Implement code generation for stmt {:?}", stmt);
            }
        }
        Ok(())
    }

    fn generate_fun_stmt(&mut self, stmt: &FunStmt) -> FelicoResult<()> {
        writeln!(self.output_file, "int _{}() {{", stmt.name.lexeme())?;
        self.generate_expr(&stmt.body)?;
        writeln!(self.output_file, "}}")?;
        Ok(())
    }

    fn generate_expr(&mut self, expr: &AstNode<Expr>) -> FelicoResult<()> {
        match &*expr.data {
            Expr::Literal(literal_expr) => {
                self.generate_literal_expr(literal_expr)?;
            }
            Expr::Call(call_expr) => {
                self.generate_call_expr(call_expr)?;
            }
            Expr::Block(block_expr) => {
                self.generate_block_expr(block_expr)?;
            }
            Expr::Variable(var_use_expr) => {
                self.generate_var_use_expr(var_use_expr)?;
            }
            _ => {
                bail!("Implement code generation for expr {:?}", expr);
            }
        }
        Ok(())
    }

    fn generate_literal_expr(&mut self, literal_expr: &LiteralExpr) -> FelicoResult<()> {
        match literal_expr {
            LiteralExpr::Str(string) => {
                write!(self.output_file, "\"{}\"", string)?;
            }
            _ => {
                bail!("Implement code generation for literal {:?}", literal_expr);
            }
        }
        Ok(())
    }

    fn generate_block_expr(&mut self, block_expr: &BlockExpr) -> FelicoResult<()> {
        for stmt in &block_expr.stmts {
            self.generate_stmt(stmt)?;
        }
        Ok(())
    }

    fn generate_call_expr(&mut self, call_expr: &CallExpr) -> FelicoResult<()> {
        self.generate_expr(&call_expr.callee)?;
        write!(self.output_file, "(")?;
        for (pos, argument) in call_expr.arguments.iter().with_position() {
            self.generate_expr(argument)?;
            if pos != Position::Last && pos != Position::Only {
                write!(self.output_file, ",")?;
            }
        }
        Ok(())
    }

    fn generate_var_use_expr(&mut self, var_use_expr: &VarUse) -> FelicoResult<()> {
        write!(self.output_file, "{}", var_use_expr.name)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::backend::c::c_code_generator::CCodeGenerator;
    use crate::frontend::parse::parser::Parser;
    use crate::frontend::resolve::resolver::Resolver;
    use crate::infra::arena::Arena;
    use crate::model::workspace::Workspace;
    use expect_test::expect;
    use std::path::Path;

    #[test]
    fn test_code_generator() {
        let arena = Arena::new();
        let workspace = Workspace::new(&arena);
        let source_file =
            workspace.source_file_from_string("hello.c", r#"debug_print("Hello World!");"#);
        let mut parser = Parser::new(source_file, workspace).unwrap();
        let mut ast = parser.parse_script().unwrap();
        let mut resolver = Resolver::new(workspace);
        resolver.resolve_program(&mut ast).unwrap();

        let output_path = Path::new("target/felico/hello.c");
        std::fs::create_dir_all(output_path.parent().unwrap()).unwrap();
        let mut code_generator = CCodeGenerator::new(&ast, output_path).unwrap();
        code_generator.generate_code().unwrap();

        expect![[r#"

        
                    #include <stdio.h>"
        
                    int main(int argc, char **argv) {
                        fprintf(stderr, "Hello, World!\n");
                        return 123;
                    }

                    int _main() {
            debug_print("Hello World!"}
        "#]]
        .assert_eq(&std::fs::read_to_string(output_path).unwrap())
    }
}
