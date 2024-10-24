use crate::frontend::ast::expr::{
    AssignExpr, BinaryExpr, BlockExpr, CallExpr, Expr, IfExpr, LiteralExpr, UnaryExpr, VarUse,
};
use crate::frontend::ast::module::Module;
use crate::frontend::ast::node::AstNode;
use crate::frontend::ast::stmt::{FunStmt, LetStmt, Stmt, WhileStmt};
use crate::frontend::lex::token::TokenType;
use crate::infra::full_name::FullName;
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
        let main_name = fullname_to_c(self.ast.data.name)? + "__main";
        self.output_file.write_all(
            r#"

#ifdef _WIN32
// needed for CLRF fix
#include <fcntl.h>
#include <io.h>
#endif

        #include <stdio.h>
        #include <math.h>

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
  return printf("%g", f);
}

int debug_print_bool(_Bool b)
{
  return printf("%s", b?"true":"false");
}

int debug_print_unknown(...)
{
  return printf("ERROR: Unknown type\n");
}
        
// Generated code
"#
            .as_bytes(),
        )?;
        self.generate_module(self.ast)?;
        write!(
            self.output_file,
            "
int main(int argc, char **argv) {{
    // Prevent Windows from rewriting LF as CRLF
#ifdef _WIN32
    _setmode(_fileno(stdout), _O_BINARY);
#endif
    {main_name}();
    return 0;
}}
"
        )?;
        Ok(())
    }

    fn generate_module(&mut self, ast: &AstNode<'ws, Module<'ws>>) -> FelicoResult<()> {
        for stmt in &ast.data.stmts {
            self.generate_stmt(stmt)?;
        }
        Ok(())
    }

    fn generate_stmt(&mut self, stmt: &AstNode<Stmt>) -> FelicoResult<()> {
        writeln!(
            self.output_file,
            "\t// {}:{}",
            stmt.location.source_file.filename(),
            stmt.location.get_line_number()
        )?;
        write!(self.output_file, "\t")?;
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
            Stmt::While(while_stmt) => {
                self.generate_while_stmt(while_stmt)?;
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
        write!(self.output_file, "{} {} = ", c_type, stmt.name.lexeme())?;
        self.generate_expr(&stmt.expression)?;
        Ok(())
    }

    fn generate_fun_stmt(&mut self, stmt: &FunStmt) -> FelicoResult<()> {
        writeln!(
            self.output_file,
            "\nint {}() {{",
            fullname_to_c(stmt.full_name)?
        )?;
        self.generate_expr(&stmt.body)?;
        write!(self.output_file, "}}")?;
        Ok(())
    }

    fn generate_while_stmt(&mut self, while_stmt: &WhileStmt) -> FelicoResult<()> {
        write!(self.output_file, "while(")?;
        self.generate_expr(&while_stmt.condition)?;
        write!(self.output_file, ")")?;
        self.generate_stmt(&while_stmt.body_stmt)?;
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
                self.generate_var_use_expr(var_use_expr, expr)?;
            }
            Expr::Binary(binary_expr) => {
                self.generate_binary_expr(binary_expr)?;
            }
            Expr::Unary(unary_expr) => {
                self.generate_unary_expr(unary_expr)?;
            }
            Expr::Assign(assign_expr) => {
                self.generate_assign_expr(assign_expr)?;
            }
            Expr::If(if_expr) => {
                self.generate_if_expr(if_expr)?;
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
                write!(self.output_file, "\"{}\"", string.escape_default())?;
            }
            LiteralExpr::F64(number) => {
                write!(self.output_file, "({:?})", number)?;
            }
            LiteralExpr::I64(number) => {
                write!(self.output_file, "{}LL", number)?;
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

    fn generate_if_expr(&mut self, if_expr: &IfExpr) -> FelicoResult<()> {
        write!(self.output_file, "(")?;
        self.generate_expr(&if_expr.condition)?;
        write!(self.output_file, ") ? (")?;
        self.generate_expr(&if_expr.then_expr)?;
        write!(self.output_file, ") : (")?;
        if let Some(else_expr) = &if_expr.else_expr {
            self.generate_expr(else_expr)?;
        } else {
            write!(self.output_file, "0")?;
        }
        write!(self.output_file, ")")?;
        Ok(())
    }

    fn generate_binary_expr(&mut self, binary_expr: &BinaryExpr) -> FelicoResult<()> {
        let operator = match binary_expr.operator.token_type() {
            TokenType::Plus => "+",
            TokenType::Minus => "-",
            TokenType::Star => "*",
            TokenType::Slash => "/",
            TokenType::And => "&&",
            TokenType::Or => "||",
            TokenType::EqualEqual => "==",
            TokenType::Greater => ">",
            TokenType::GreaterEqual => ">=",
            TokenType::Less => "<",
            TokenType::LessEqual => "<=",
            _ => {
                bail!(
                    "Implement code generation for binary expr {:?}",
                    binary_expr
                );
            }
        };
        write!(self.output_file, "(")?;
        self.generate_expr(&binary_expr.left)?;
        write!(self.output_file, " ")?;
        self.output_file.write_all(operator.as_bytes())?;
        write!(self.output_file, " ")?;
        self.generate_expr(&binary_expr.right)?;
        write!(self.output_file, ")")?;
        Ok(())
    }

    fn generate_unary_expr(&mut self, unary_expr: &UnaryExpr) -> FelicoResult<()> {
        let operator = match unary_expr.operator.token_type() {
            TokenType::Plus => "+",
            TokenType::Minus => "-",
            _ => {
                bail!("Implement code generation for unary expr {:?}", unary_expr);
            }
        };
        write!(self.output_file, "(")?;
        self.output_file.write_all(operator.as_bytes())?;
        self.generate_expr(&unary_expr.right)?;
        write!(self.output_file, ")")?;
        Ok(())
    }

    fn generate_block_expr(&mut self, block_expr: &BlockExpr) -> FelicoResult<()> {
        writeln!(self.output_file, "{{")?;
        for stmt in &block_expr.stmts {
            self.generate_stmt(stmt)?;
        }
        writeln!(self.output_file, "}}")?;
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

    fn generate_var_use_expr(
        &mut self,
        var_use_expr: &VarUse,
        expr: &AstNode<Expr>,
    ) -> FelicoResult<()> {
        if matches!(expr.ty.kind(), TypeKind::Type) {
            write!(
                self.output_file,
                "\"❬{}❭\"",
                var_use_expr
                    .name
                    .data
                    .parts
                    .iter()
                    .map(|part| part.lexeme())
                    .join("::")
            )?;
        } else {
            write!(
                self.output_file,
                "{}",
                fullname_to_c(var_use_expr.name.data.full_name)?
            )?;
        }
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

fn fullname_to_c(full_name: FullName) -> FelicoResult<String> {
    if full_name.is_unresolved() {
        bail!("Fullname is unresolved");
    }
    let c_name = full_name.parts().join("__");
    Ok(c_name)
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

            	// hello.c:1
	
            int hello__main() {
            {
            	// hello.c:1
            	debug_print("Hello World!");
            }
            };

            int main(int argc, char **argv) {
                // Prevent Windows from rewriting LF as CRLF
            #ifdef _WIN32
                _setmode(_fileno(stdout), _O_BINARY);
            #endif
                hello__main();
                return 0;
            }
        "#]]
        .assert_eq(
            &std::fs::read_to_string(output_path)
                .unwrap()
                .rsplit_once("// Generated code")
                .unwrap()
                .1,
        )
    }
}
