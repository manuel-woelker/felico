use crate::frontend::ast::expr::{
    AssignExpr, BinaryExpr, BlockExpr, CallExpr, Expr, LiteralExpr, VarUse,
};
use crate::frontend::ast::module::Module;
use crate::frontend::ast::node::AstNode;
use crate::frontend::ast::stmt::{FunStmt, LetStmt, Stmt};
use crate::frontend::lex::token::TokenType;
use crate::infra::result::{bail, FelicoResult};
use crate::model::types::{PrimitiveType, Type, TypeKind};
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
        self.output_file.write_all(
            r#"

        #include <stdio.h>

#define debug_print(X) _Generic((X), int: debug_print_int, \
                              char*: debug_print_string, \
                              _Bool: debug_print_bool, \
                              double: debug_print_double \
                              )(X)

int debug_print_string(char* string)
{
  return printf("%s", string);
}

int debug_print_int(int i)
{
  return printf("%d", i);
}

int debug_print_double(double f)
{
  return printf("%f", f);
}

int debug_print_bool(_Bool b)
{
  return printf("%s", b?"true":"false");
}

int debug_print_unknown(...)
{
  return printf("ERROR: Unknown type\n");
}
        
        int _main();
    
        int main(int argc, char **argv) {
            _main();
            return 0;
        }

        "#
            .as_bytes(),
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
            Stmt::Let(let_stmt) => {
                self.generate_let_stmt(let_stmt, stmt)?;
            }
            _ => {
                bail!("Implement code generation for stmt {:?}", stmt);
            }
        }
        self.output_file.write_all(";\n".as_bytes())?;
        Ok(())
    }

    fn generate_let_stmt(&mut self, stmt: &LetStmt, ast: &AstNode<Stmt>) -> FelicoResult<()> {
        let c_type = self.get_c_type(ast.ty)?;
        writeln!(self.output_file, "{} {} = ", c_type, stmt.name.lexeme())?;
        self.generate_expr(&stmt.expression)?;
        writeln!(self.output_file, ";")?;
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
            Expr::Binary(binary_expr) => {
                self.generate_binary_expr(binary_expr)?;
            }
            Expr::Assign(assign_expr) => {
                self.generate_assign_expr(assign_expr)?;
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
            LiteralExpr::F64(number) => {
                write!(self.output_file, "((double){})", number)?;
            }
            LiteralExpr::I64(number) => {
                write!(self.output_file, "((int64_t){})", number)?;
            }
            LiteralExpr::Bool(bool) => {
                write!(self.output_file, "{}", bool)?;
            }
            LiteralExpr::Unit => {}
        }
        Ok(())
    }

    fn generate_assign_expr(&mut self, assign_expr: &AssignExpr) -> FelicoResult<()> {
        write!(self.output_file, "({} =", &assign_expr.destination)?;
        self.generate_expr(&assign_expr.value)?;
        write!(self.output_file, ")")?;
        Ok(())
    }

    fn generate_binary_expr(&mut self, binary_expr: &BinaryExpr) -> FelicoResult<()> {
        let operator = match binary_expr.operator.token_type() {
            TokenType::Plus => "+",
            TokenType::Minus => "-",
            TokenType::Star => "*",
            TokenType::Slash => "/",
            _ => {
                bail!(
                    "Implement code generation for binary expr {:?}",
                    binary_expr
                );
            }
        };
        write!(self.output_file, "(")?;
        self.generate_expr(&binary_expr.left)?;
        self.output_file.write_all(operator.as_bytes())?;
        self.generate_expr(&binary_expr.right)?;
        write!(self.output_file, ")")?;
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
        write!(self.output_file, ")")?;
        Ok(())
    }

    fn generate_var_use_expr(&mut self, var_use_expr: &VarUse) -> FelicoResult<()> {
        write!(self.output_file, "{}", var_use_expr.name)?;
        Ok(())
    }

    fn get_c_type(&self, ty: Type) -> FelicoResult<&'ws str> {
        match ty.kind() {
            TypeKind::Primitive(primitive) => Ok(match primitive {
                PrimitiveType::Bool => "_Bool",
                PrimitiveType::F64 => "double",
                PrimitiveType::I64 => "int64_t",
                PrimitiveType::Str => "char*",
            }),
            _ => {
                bail!("Implement get_c_type for type {}", ty);
            }
        }
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

        
                    #include <stdio.h>
                    
                    int debug_print(char* message) {
                        printf(message);
                    }
        
                    int main(int argc, char **argv) {
                        fprintf(stderr, "Hello, World!\n");
                        return 123;
                    }

                    int _main() {
            debug_print("Hello World!");
            }
            ;
        "#]]
        .assert_eq(&std::fs::read_to_string(output_path).unwrap())
    }
}
