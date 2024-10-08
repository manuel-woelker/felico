use crate::frontend::ast::expr::Expr;
use crate::frontend::ast::node::AstNode;
use crate::frontend::ast::program::Program;
use crate::frontend::ast::stmt::Stmt;
use crate::frontend::ast::AstData;
use crate::infra::result::FelicoResult;
use std::io::{BufWriter, Cursor, Write};
use termtree::Tree;

pub struct AstPrinter {
    print_locations: bool,
    print_types: bool,
}

impl AstPrinter {
    pub fn new() -> Self {
        Self {
            print_locations: true,
            print_types: false,
        }
    }

    pub fn with_types(mut self, on: bool) -> Self {
        self.print_types = on;
        self
    }

    pub fn with_locations(mut self, on: bool) -> Self {
        self.print_locations = on;
        self
    }

    fn using_worker(
        &self,
        print_fn: impl FnOnce(&mut AstPrinterWorker) -> FelicoResult<()>,
    ) -> FelicoResult<String> {
        let mut buffer = Cursor::new(Vec::<u8>::new());
        {
            let mut worker = AstPrinterWorker {
                write: BufWriter::new(&mut buffer),
                print_locations: self.print_locations,
                print_types: self.print_types,
            };
            print_fn(&mut worker)?;
        }
        let printed_ast = String::from_utf8(buffer.into_inner()).unwrap();
        Ok(printed_ast)
    }

    pub fn print(&self, ast: &AstNode<Program>) -> FelicoResult<String> {
        self.using_worker(|worker| worker.print_program(ast))
    }

    pub fn print_expr(&self, ast: &AstNode<Expr>) -> FelicoResult<String> {
        self.using_worker(|worker| worker.print_expr(ast))
    }
}

pub fn ast_to_string(ast: &AstNode<Program>) -> FelicoResult<String> {
    AstPrinter::new().print(ast)
}

struct AstPrinterWorker<'a> {
    write: BufWriter<&'a mut dyn Write>,
    print_locations: bool,
    print_types: bool,
}

impl<'a> AstPrinterWorker<'a> {
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
            Expr::Block(block) => {
                let mut tree = Tree::new("Block".into());
                for stmt in &block.stmts {
                    tree.push(self.stmt_to_tree(stmt));
                }
                tree.push(self.expr_to_tree(&block.result_expression));
                tree
            }
            Expr::If(if_expr) => {
                let mut tree = Tree::new("If".into());
                tree.push(self.expr_to_tree(&if_expr.condition));
                tree.push(self.expr_to_tree(&if_expr.then_expr));
                if let Some(else_expr) = &if_expr.else_expr {
                    tree.push(self.expr_to_tree(else_expr));
                }
                tree
            }
            Expr::Return(return_expr) => {
                let mut tree = Tree::new("Return".into());
                tree.push(self.expr_to_tree(&return_expr.expression));
                tree
            }
        };
        self.add_location(tree, ast)
    }

    fn stmt_to_tree(&self, ast: &AstNode<Stmt>) -> Tree<String> {
        let tree = match &ast.data.as_ref() {
            Stmt::Expression(expr) => return self.expr_to_tree(&expr.expression),
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
                        .map(|p| p.name.lexeme())
                        .collect::<Vec<&str>>()
                        .join(", ")
                ));
                for parameter in &fun.parameters {
                    let mut paramtree = Tree::new(format!("Param {}", parameter.name.lexeme()));
                    paramtree.push(self.expr_to_tree(&parameter.type_expression));
                    tree.push(paramtree);
                }
                let mut return_type_tree = self.expr_to_tree(&fun.return_type);
                return_type_tree.root = "Return type: ".to_string() + &return_type_tree.root;
                tree.push(return_type_tree);
                tree.leaves.append(&mut self.expr_to_tree(&fun.body).leaves);
                tree
            }
            Stmt::Struct(struct_stmt) => {
                let mut tree = Tree::new(format!("Struct '{}'", struct_stmt.name.lexeme()));
                for field in &struct_stmt.fields {
                    let mut field_tree = Tree::new(format!("Field {}", field.data.name.lexeme()));
                    field_tree.push(self.expr_to_tree(&field.data.type_expression));
                    let field_tree = self.add_location(field_tree, field);
                    tree.push(field_tree);
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
        if self.print_locations {
            let location = &ast.location;
            tree.root += &format!(
                "     [{}+{}]",
                location.start_byte,
                location.end_byte - location.start_byte
            )
        }
        if self.print_types && !ast.ty.is_unknown() {
            tree.root += &format!(": {}", ast.ty)
        }
        tree
    }
}
