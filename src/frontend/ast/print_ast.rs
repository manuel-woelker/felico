use crate::frontend::ast::expr::Expr;
use crate::frontend::ast::node::AstNode;
use crate::frontend::ast::program::Program;
use crate::frontend::ast::stmt::Stmt;
use crate::frontend::ast::AstData;
use crate::infra::result::FelicoResult;
use std::io::{BufWriter, Cursor, Write};
use termtree::Tree;

pub fn print_expr_ast(ast: &AstNode<Expr>, write: &mut dyn Write) -> FelicoResult<()> {
    let mut ast_printer = AstPrinter {
        write: BufWriter::new(write),
        include_locations: true,
    };
    ast_printer.print_expr(ast)
}

pub fn print_program_ast(ast: &AstNode<Program>, write: &mut dyn Write) -> FelicoResult<()> {
    let mut ast_printer = AstPrinter {
        write: BufWriter::new(write),
        include_locations: true,
    };
    ast_printer.print_program(ast)
}

pub fn ast_to_string(ast: &AstNode<Program>) -> FelicoResult<String> {
    let mut buffer = Cursor::new(Vec::<u8>::new());
    print_program_ast(ast, &mut buffer)?;
    Ok(String::from_utf8(buffer.into_inner()).unwrap())
}

struct AstPrinter<'a> {
    write: BufWriter<&'a mut dyn Write>,
    include_locations: bool,
}

impl<'a> AstPrinter<'a> {
    fn print_expr(&mut self, ast: &AstNode<Expr>) -> FelicoResult<()> {
        let tree = self.expr_to_tree(ast);
        write!(self.write, "{}", tree)?;
        Ok(())
    }

    fn print_program(&mut self, ast: &AstNode<Program>) -> FelicoResult<()> {
        let tree = self.program_to_tree(ast);
        write!(self.write, "{}", tree)?;
        Ok(())
    }

    fn expr_to_tree(&self, ast: &AstNode<Expr>) -> Tree<String> {
        let tree = match &ast.data.as_ref() {
            Expr::Literal(literal) => Tree::new(format!("{:?}", literal)),
            Expr::Variable(var_use) => Tree::new(format!("Read '{}'", var_use.variable.lexeme())),
            Expr::Unary(unary) => {
                let mut tree = Tree::new(unary.operator.lexeme().to_string());
                tree.push(self.expr_to_tree(&unary.right));
                tree
            }
            Expr::Binary(binary) => {
                let mut tree = Tree::new(binary.operator.lexeme().to_string());
                tree.push(self.expr_to_tree(&binary.left));
                tree.push(self.expr_to_tree(&binary.right));
                tree
            }
            Expr::Assign(assign) => {
                let mut tree = Tree::new(format!("{} = ", assign.destination));
                tree.push(self.expr_to_tree(&assign.value));
                tree
            }
            Expr::Call(call) => {
                let mut tree = Tree::new("Call".into());
                tree.push(self.expr_to_tree(&call.callee));
                for expr in &call.arguments {
                    tree.push(self.expr_to_tree(expr));
                }
                tree
            }
            Expr::Get(get) => {
                let mut tree = Tree::new(format!("Get {}", get.name.lexeme()));
                tree.push(self.expr_to_tree(&get.object));
                tree
            }
            Expr::Set(set) => {
                let mut tree = Tree::new(format!("Set {}", set.name.lexeme()));
                tree.push(self.expr_to_tree(&set.object));
                tree.push(self.expr_to_tree(&set.value));
                tree
            }
        };
        self.add_location(tree, ast)
    }

    fn stmt_to_tree(&self, ast: &AstNode<Stmt>) -> Tree<String> {
        let tree = match &ast.data.as_ref() {
            Stmt::Expression(expr) => self.expr_to_tree(&expr.expression),
            Stmt::Return(return_stmt) => {
                let mut tree = Tree::new("Return".into());
                tree.push(self.expr_to_tree(&return_stmt.expression));
                tree
            }
            Stmt::Let(var) => {
                let mut tree = Tree::new(format!("Let '{}'", var.name));
                tree.push(self.expr_to_tree(&var.expression));
                tree
            }
            Stmt::Fun(fun) => {
                let mut tree = Tree::new(format!(
                    "Declare fun '{}({})'",
                    fun.name.lexeme(),
                    fun.parameters
                        .iter()
                        .map(|p| p.lexeme())
                        .collect::<Vec<&str>>()
                        .join(",")
                ));
                tree.leaves.append(&mut self.stmt_to_tree(&fun.body).leaves);
                tree
            }
            Stmt::Block(block) => {
                let mut tree = Tree::new("Block".into());
                for stmt in &block.stmts {
                    tree.push(self.stmt_to_tree(stmt));
                }
                tree
            }
            Stmt::If(if_stmt) => {
                let mut tree = Tree::new("If".into());
                tree.push(self.expr_to_tree(&if_stmt.condition));
                tree.push(self.stmt_to_tree(&if_stmt.then_stmt));
                if let Some(else_stmt) = &if_stmt.else_stmt {
                    tree.push(self.stmt_to_tree(else_stmt));
                }
                tree
            }
            Stmt::While(while_stmt) => {
                let mut tree = Tree::new("While".into());
                tree.push(self.expr_to_tree(&while_stmt.condition));
                tree.push(self.stmt_to_tree(&while_stmt.body_stmt));
                tree
            }
        };
        self.add_location(tree, ast)
    }

    fn program_to_tree(&self, ast: &AstNode<Program>) -> Tree<String> {
        let mut tree = Tree::new("Program".into());
        for stmt in &ast.data.stmts {
            tree.push(self.stmt_to_tree(stmt));
        }
        tree
    }

    fn add_location<T: AstData>(&self, mut tree: Tree<String>, ast: &AstNode<T>) -> Tree<String> {
        if self.include_locations {
            let location = &ast.location;
            tree.root += &format!(
                "     [{}+{}]",
                location.start_byte,
                location.end_byte - location.start_byte
            )
        }
        tree
    }
}
