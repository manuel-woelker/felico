use crate::frontend::ast::expr::{Expr, LiteralExpr};
use crate::frontend::ast::node::AstNode;
use crate::frontend::ast::program::Program;
use crate::frontend::ast::stmt::Stmt;
use crate::frontend::ast::stmt::Stmt::Let;
use crate::frontend::lexer::token::Token;
use crate::infra::diagnostic::InterpreterDiagnostic;
use crate::infra::location::Location;
use crate::infra::result::{bail, FelicoResult};
use crate::infra::source_file::SourceFileHandle;
use crate::interpreter::core_definitions::{get_core_definitions, TYPE_BOOL, TYPE_F64, TYPE_FUNCTION, TYPE_STRING, TYPE_UNIT, TYPE_UNKNOWN};
use crate::interpreter::value::{InterpreterValue, Type, ValueKind};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::ops::DerefMut;

struct Symbol {
    declaration_site: Location,
    is_defined: bool,
    ty: Type,
    value: Option<InterpreterValue>,
}


struct ResolverPass {
    scopes: Vec<HashMap<String, Symbol>>,
}

impl ResolverPass {
    fn new() -> Self {
        let mut global_scope: HashMap<String, Symbol> = Default::default();
        let location = Location {
            source_file: SourceFileHandle::from_string("native", "native_code"),
            start_byte: 0,
            end_byte: 0,
        };
        for core_definition in get_core_definitions() {
            global_scope.insert(core_definition.name.to_string(), Symbol {
                declaration_site: location.clone(),
                is_defined: true,
                ty: core_definition.value.ty.clone(),
                value: Some(core_definition.value.clone()),
            });
        }
        ResolverPass {
            scopes: vec![global_scope, Default::default()],
        }
    }
    pub(crate) fn resolve_program(&mut self, program: &mut AstNode<Program>) -> FelicoResult<()> {
        self.resolve_stmts(&mut program.data.stmts)
    }

    fn resolve_stmts(&mut self, stmts: &mut Vec<AstNode<Stmt>>) -> FelicoResult<()> {
        for stmt in stmts {
            self.resolve_stmt(stmt)?;
        }
        Ok(())
    }

    fn resolve_stmt(&mut self, stmt: &mut AstNode<Stmt>) -> FelicoResult<()> {
        match stmt.data.deref_mut() {
            Let(let_stmt) => {
                let name = let_stmt.name.lexeme();
                let ty = if let Some(expr) = &let_stmt.type_expression {
                    if let Expr::Variable(type_id) = &*expr.data {
                        let distance_and_symbol = self.get_definition_distance_and_symbol(&type_id.variable);
                        if let Some((_distance, symbol)) = distance_and_symbol {
                            if let Some(value) = &symbol.value {
                                if let ValueKind::Type(ty) = &value.val {
                                    ty.clone()
                                } else {
                                    bail!("Type expression must be a type: {}", type_id.variable.lexeme());
                                }
                            } else {
                                bail!("Unknown value for symbol: {}", type_id.variable.lexeme());
                            }
                        } else {
                            bail!("Unsupported type: {}", type_id.variable.lexeme());
                        }
                    } else {
                        bail!("Unsupported expression in type position: {:?}", expr);
                    }
                } else {
                    TYPE_UNKNOWN.clone()
                };
                match self.current_scope().entry(name.to_string()) {
                    Entry::Occupied(value) => {
                        let mut diagnostic = InterpreterDiagnostic::new(&stmt.location.source_file, format!("Variable '{}' already declared", let_stmt.name.lexeme()));
                        diagnostic.add_primary_label(&let_stmt.name.location);
                        diagnostic.add_label(&value.get().declaration_site, "is already defined here");
                        return Err(diagnostic.into());
                    }
                    Entry::Vacant(slot) => {
                        slot.insert(Symbol { declaration_site: let_stmt.name.location.clone(), is_defined: false, ty, value: None });
                    }
                }
                self.resolve_expr(&mut let_stmt.expression)?;
                self.current_scope().get_mut(name).unwrap().is_defined = true;
            }
            Stmt::Return(return_stmt) => {
                self.resolve_expr(&mut return_stmt.expression)?;
            }
            Stmt::Expression(expr_stmt) => {
                self.resolve_expr(&mut expr_stmt.expression)?;
            }
            Stmt::Fun(fun_stmt) => {
                let name = fun_stmt.name.lexeme();
                match self.current_scope().entry(name.to_string()) {
                    Entry::Occupied(value) => {
                        let mut diagnostic = InterpreterDiagnostic::new(&stmt.location.source_file, format!("Function '{}' already declared", fun_stmt.name.lexeme()));
                        diagnostic.add_primary_label(&fun_stmt.name.location);
                        diagnostic.add_label(&value.get().declaration_site, "is already defined here");
                        return Err(diagnostic.into());
                    }
                    Entry::Vacant(slot) => {
                        slot.insert(Symbol { declaration_site: fun_stmt.name.location.clone(), is_defined: true, ty: TYPE_FUNCTION.clone(), value: None });
                    }
                }
                self.scopes.push(Default::default());
                let current_scope = self.current_scope();
                for parameter in &fun_stmt.parameters {
                    current_scope.insert(parameter.lexeme().to_string(), Symbol { declaration_site: parameter.location.clone(), is_defined: true, ty: TYPE_UNKNOWN.clone(), value: None });
                }
                self.resolve_stmt(&mut fun_stmt.body)?;
                self.scopes.pop();
                self.current_scope().get_mut(name).unwrap().is_defined = true;
            }
            Stmt::Block(block) => {
                self.scopes.push(Default::default());
                self.resolve_stmts(&mut block.stmts)?;
                self.scopes.pop();
            }
            Stmt::If(if_stmt) => {
                self.resolve_expr(&mut if_stmt.condition)?;
                self.resolve_stmt(&mut if_stmt.then_stmt)?;
                if let Some(stmt) = &mut if_stmt.else_stmt {
                    self.resolve_stmt(stmt)?;
                }
            }
            Stmt::While(while_stmt) => {
                self.resolve_expr(&mut while_stmt.condition)?;
                self.resolve_stmt(&mut while_stmt.body_stmt)?;
            }
        }
        Ok(())
    }

    fn resolve_expr(&mut self, expr: &mut AstNode<Expr>) -> FelicoResult<()> {
        match expr.data.deref_mut() {
            Expr::Unary(unary) => {
                self.resolve_expr(&mut unary.right)?;
            }
            Expr::Binary(binary) => {
                self.resolve_expr(&mut binary.left)?;
                self.resolve_expr(&mut binary.right)?;
            }
            Expr::Literal(literal) => {
                expr.ty = match literal {
                    LiteralExpr::String(_) => {
                        TYPE_STRING.clone()
                    }
                    LiteralExpr::Number(_) => {
                        TYPE_F64.clone()
                    }
                    LiteralExpr::Bool(_) => {
                        TYPE_BOOL.clone()
                    }
                    LiteralExpr::Unit => {
                        TYPE_UNIT.clone()
                    }
                }
            }
            Expr::Variable(var_use) => {
                let distance_and_symbol = self.get_definition_distance_and_symbol(&var_use.variable);
                if let Some((distance, _)) = distance_and_symbol {
                    var_use.distance = distance;
                } else {
                    let mut diagnostic = InterpreterDiagnostic::new(&var_use.variable.location.source_file, format!("Variable '{}' is not defined here", var_use.variable.lexeme()));
                    diagnostic.add_primary_label(&var_use.variable.location);
                    return Err(diagnostic.into());
                }
            }
            Expr::Assign(assign) => {
                let destination = &assign.destination;
                self.resolve_expr(&mut assign.value)?;
                let distance_and_symbol = self.get_definition_distance_and_symbol(&destination);
                if let Some((distance, symbol)) = distance_and_symbol {
                    assign.distance = distance;
                    let destination_type = &symbol.ty;
                    if destination_type != &*TYPE_UNKNOWN {
                        let expression_type = &assign.value.ty;
                        if destination_type != &*expression_type {
                            let mut diagnostic = InterpreterDiagnostic::new(&destination.location.source_file, format!("Expression value of type {} cannot be assigned to variable '{}' of type {}", expression_type, assign.destination.lexeme(), destination_type));
                            diagnostic.add_primary_label(&expr.location);
                            diagnostic.add_label(&symbol.declaration_site, format!("is declared as {} here", destination_type));
                            return Err(diagnostic.into());
                        }
                    }
                } else {
                    let mut diagnostic = InterpreterDiagnostic::new(&destination.location.source_file, format!("Variable '{}' is not defined here", destination.lexeme()));
                    diagnostic.add_primary_label(&destination.location);
                    return Err(diagnostic.into());
                }
            }
            Expr::Call(call) => {
                self.resolve_expr(&mut call.callee)?;
                for arg in &mut call.arguments {
                    self.resolve_expr(arg)?
                }
            }
            Expr::Get(get) => {
                self.resolve_expr(&mut get.object)?;
            }
            Expr::Set(set) => {
                self.resolve_expr(&mut set.value)?;
                self.resolve_expr(&mut set.object)?;
            }
        }
        Ok(())
    }

    fn current_scope(&mut self) -> &mut HashMap<String, Symbol> {
        self.scopes.iter_mut().last().expect("Scope Stack should not be empty")
    }

    fn get_definition_distance_and_symbol(&self, variable: &Token) -> Option<(i32, &Symbol)> {
        let name = variable.lexeme();
        let mut distance = -1;
        for scope in self.scopes.iter().rev() {
            distance += 1;
            if let Some(entry) = scope.get(name) {
                if entry.is_defined {
                    return Some((distance, entry));
                }
            }
        }
        None
    }
}

pub fn resolve_variables(ast: &mut AstNode<Program>) -> FelicoResult<()> {
    ResolverPass::new().resolve_program(ast)
}

#[cfg(test)]
mod tests {
    use crate::frontend::parser::parser::Parser;
    use crate::frontend::resolver::resolver_pass::resolve_variables;
    use crate::infra::diagnostic::unwrap_diagnostic_to_string;
    use expect_test::{expect, Expect};

    fn test_resolve_program_error(name: &str, input: &str, expected: Expect) {
        let parser = Parser::new_in_memory(name, input).unwrap();
        let mut ast = parser.parse_program().unwrap();
        let result = resolve_variables(&mut ast);
        let diagnostic_string = unwrap_diagnostic_to_string(&result);
        expected.assert_eq(&diagnostic_string);
    }

    macro_rules! test_resolve_error {
    ( $($label:ident: $input:expr => $expect:expr;)+ ) => {
        $(
            #[test]
            fn $label() {
                test_resolve_program_error(stringify!($label), $input, $expect);
            }
        )*
        }
    }

    test_resolve_error!(
        double_declaration: "let x = 0;\ndebug_print(x);\nlet x = true;" => expect![[r#"
            × Variable 'x' already declared
               ╭─[double_declaration:3:5]
             1 │ let x = 0;
               ·     ┬
               ·     ╰── is already defined here
             2 │ debug_print(x);
             3 │ let x = true;
               ·     ─
               ╰────"#]];
        use_undefined_variable: "debug_print(x);" => expect![[r#"
            × Variable 'x' is not defined here
               ╭─[use_undefined_variable:1:13]
             1 │ debug_print(x);
               ·             ─
               ╰────"#]];
        assign_undefined_variable: "x = 3;" => expect![[r#"
            × Variable 'x' is not defined here
               ╭─[assign_undefined_variable:1:1]
             1 │ x = 3;
               · ─
               ╰────"#]];
        call_undefined_function: "x();" => expect![[r#"
            × Variable 'x' is not defined here
               ╭─[call_undefined_function:1:1]
             1 │ x();
               · ─
               ╰────"#]];
        assign_wrong_type_to_variable: r#"
        fun f() {
            let x: bool = true;
            x=3;
         }"# => expect![[r#"
             × Expression value of type ❬f64❭ cannot be assigned to variable 'x' of type ❬bool❭
                ╭─[assign_wrong_type_to_variable:4:13]
              2 │         fun f() {
              3 │             let x: bool = true;
                ·                 ┬
                ·                 ╰── is declared as ❬bool❭ here
              4 │             x=3;
                ·             ────
              5 │          }
                ╰────"#]];
    );
}