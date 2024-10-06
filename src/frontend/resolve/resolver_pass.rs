use crate::frontend::ast::expr::{Expr, LiteralExpr};
use crate::frontend::ast::node::AstNode;
use crate::frontend::ast::program::Program;
use crate::frontend::ast::stmt::Stmt;
use crate::frontend::ast::stmt::Stmt::Let;
use crate::frontend::ast::types::{StructField, Type, TypeKind};
use crate::frontend::lex::token::Token;
use crate::frontend::resolve::type_checker::TypeChecker;
use crate::infra::diagnostic::InterpreterDiagnostic;
use crate::infra::location::Location;
use crate::infra::result::{bail, FelicoResult};
use crate::infra::shared_string::SharedString;
use crate::infra::source_file::SourceFileHandle;
use crate::interpret::core_definitions::{get_core_definitions, TypeFactory};
use crate::interpret::value::{InterpreterValue, ValueKind};
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
    type_factory: TypeFactory,
    type_checker: TypeChecker,
}

impl ResolverPass {
    fn new(type_factory: TypeFactory) -> Self {
        let mut global_scope: HashMap<String, Symbol> = Default::default();
        let location = Location {
            source_file: SourceFileHandle::from_string("native", "native_code"),
            start_byte: 0,
            end_byte: 0,
        };
        for core_definition in get_core_definitions(&type_factory) {
            global_scope.insert(
                core_definition.name.to_string(),
                Symbol {
                    declaration_site: location.clone(),
                    is_defined: true,
                    ty: core_definition.value.ty.clone(),
                    value: Some(core_definition.value.clone()),
                },
            );
        }
        ResolverPass {
            scopes: vec![global_scope, Default::default()],
            type_factory,
            type_checker: TypeChecker::new(),
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
        let type_factory = self.type_factory.clone();
        match stmt.data.deref_mut() {
            Let(let_stmt) => {
                let name = let_stmt.name.lexeme();
                self.resolve_expr(&mut let_stmt.expression)?;
                let expression_type = &let_stmt.expression.ty;
                let variable_type = if let Some(type_expr) = &let_stmt.type_expression {
                    self.resolve_type(&type_expr)?
                } else {
                    let_stmt.expression.ty.clone()
                };
                if !self
                    .type_checker
                    .is_assignable_to(&expression_type, &variable_type)
                {
                    let mut diagnostic = InterpreterDiagnostic::new(&stmt.location.source_file, format!("Expression value of type {} cannot be assigned to variable '{}' declared to be type {}", expression_type, let_stmt.name.lexeme(), variable_type));
                    diagnostic.add_primary_label(&stmt.location);
                    return Err(diagnostic.into());
                }
                stmt.ty = variable_type.clone();
                match self.current_scope().entry(name.to_string()) {
                    Entry::Occupied(value) => {
                        let mut diagnostic = InterpreterDiagnostic::new(
                            &stmt.location.source_file,
                            format!("Variable '{}' already declared", let_stmt.name.lexeme()),
                        );
                        diagnostic.add_primary_label(&let_stmt.name.location);
                        diagnostic
                            .add_label(&value.get().declaration_site, "is already defined here");
                        return Err(diagnostic.into());
                    }
                    Entry::Vacant(slot) => {
                        slot.insert(Symbol {
                            declaration_site: let_stmt.name.location.clone(),
                            is_defined: false,
                            ty: variable_type,
                            value: None,
                        });
                    }
                }
                self.current_scope().get_mut(name).unwrap().is_defined = true;
            }
            Stmt::Return(return_stmt) => {
                self.resolve_expr(&mut return_stmt.expression)?;
            }
            Stmt::Expression(expr_stmt) => {
                self.resolve_expr(&mut expr_stmt.expression)?;
            }
            Stmt::Struct(struct_stmt) => {
                let mut fields = HashMap::new();
                for field in &mut struct_stmt.fields {
                    self.resolve_expr(&mut field.data.type_expression)?;
                    field.ty = self.resolve_type(&field.data.type_expression)?;
                    let name = SharedString::from(field.data.name.lexeme());
                    fields.insert(name.clone(), StructField::new(&field.data.name, &field.ty));
                }
                stmt.ty = type_factory.make_struct(&struct_stmt.name, fields);
                // TODO: add to symbol table
            }
            Stmt::Fun(fun_stmt) => {
                let name = fun_stmt.name.lexeme();
                let return_type = self.resolve_type(&fun_stmt.return_type)?;
                let parameter_types: Vec<Type> = fun_stmt
                    .parameters
                    .iter()
                    .map(|parameter| self.resolve_type(&parameter.type_expression))
                    .collect::<FelicoResult<Vec<_>>>()?;
                let function_type = type_factory.function(parameter_types, return_type);
                match self.current_scope().entry(name.to_string()) {
                    Entry::Occupied(value) => {
                        let mut diagnostic = InterpreterDiagnostic::new(
                            &stmt.location.source_file,
                            format!("Function '{}' already declared", fun_stmt.name.lexeme()),
                        );
                        diagnostic.add_primary_label(&fun_stmt.name.location);
                        diagnostic
                            .add_label(&value.get().declaration_site, "is already defined here");
                        return Err(diagnostic.into());
                    }
                    Entry::Vacant(slot) => {
                        stmt.ty = function_type.clone();
                        slot.insert(Symbol {
                            declaration_site: fun_stmt.name.location.clone(),
                            is_defined: true,
                            ty: function_type,
                            value: None,
                        });
                    }
                }
                self.scopes.push(Default::default());
                for parameter in &fun_stmt.parameters {
                    let ty = self.resolve_type(&parameter.type_expression)?.clone();
                    self.current_scope().insert(
                        parameter.name.lexeme().to_string(),
                        Symbol {
                            declaration_site: parameter.name.location.clone(),
                            is_defined: true,
                            ty,
                            value: None,
                        },
                    );
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

    fn resolve_type(&mut self, expr: &AstNode<Expr>) -> FelicoResult<Type> {
        // TODO: make bails into diagnostics
        let Expr::Variable(type_id) = &*expr.data else {
            bail!("Unsupported expression in type position: {:?}", expr);
        };
        let distance_and_symbol = self.get_definition_distance_and_symbol(&type_id.variable);
        let Some((_distance, symbol)) = distance_and_symbol else {
            bail!("Unknown symbol: {}", type_id.variable.lexeme());
        };
        let Some(value) = &symbol.value else {
            bail!("Unknown value for symbol: {}", type_id.variable.lexeme());
        };
        let ValueKind::Type(ty) = &value.val else {
            bail!(
                "Type expression must be a type: {}",
                type_id.variable.lexeme()
            );
        };
        Ok(ty.clone())
    }

    fn resolve_expr(&mut self, expr: &mut AstNode<Expr>) -> FelicoResult<()> {
        match expr.data.deref_mut() {
            Expr::Unary(unary) => {
                self.resolve_expr(&mut unary.right)?;
                expr.ty = unary.right.ty.clone();
            }
            Expr::Binary(binary) => {
                self.resolve_expr(&mut binary.left)?;
                self.resolve_expr(&mut binary.right)?;
                if binary.left.ty == binary.right.ty {
                    expr.ty = binary.left.ty.clone();
                }
            }
            Expr::Literal(literal) => {
                expr.ty = match literal {
                    LiteralExpr::Str(_) => self.type_factory.str(),
                    LiteralExpr::F64(_) => self.type_factory.f64(),
                    LiteralExpr::I64(_) => self.type_factory.i64(),
                    LiteralExpr::Bool(_) => self.type_factory.bool(),
                    LiteralExpr::Unit => self.type_factory.unit(),
                }
            }
            Expr::Variable(var_use) => {
                let distance_and_symbol =
                    self.get_definition_distance_and_symbol(&var_use.variable);
                if let Some((distance, symbol)) = distance_and_symbol {
                    var_use.distance = distance;
                    expr.ty = symbol.ty.clone();
                } else {
                    let mut diagnostic = InterpreterDiagnostic::new(
                        &var_use.variable.location.source_file,
                        format!(
                            "Variable '{}' is not defined here",
                            var_use.variable.lexeme()
                        ),
                    );
                    diagnostic.add_primary_label(&var_use.variable.location);
                    return Err(diagnostic.into());
                }
            }
            Expr::Assign(assign) => {
                let destination = &assign.destination;
                self.resolve_expr(&mut assign.value)?;
                let distance_and_symbol = self.get_definition_distance_and_symbol(destination);
                if let Some((distance, symbol)) = distance_and_symbol {
                    assign.distance = distance;
                    let destination_type = &symbol.ty;
                    expr.ty = symbol.ty.clone();

                    if !destination_type.is_unknown() {
                        let expression_type = &assign.value.ty;
                        if !self
                            .type_checker
                            .is_assignable_to(expression_type, destination_type)
                        {
                            let mut diagnostic = InterpreterDiagnostic::new(&destination.location.source_file, format!("Expression value of type {} cannot be assigned to variable '{}' of type {}", expression_type, assign.destination.lexeme(), destination_type));
                            diagnostic.add_primary_label(&expr.location);
                            diagnostic.add_label(
                                &symbol.declaration_site,
                                format!("is declared as {} here", destination_type),
                            );
                            return Err(diagnostic.into());
                        }
                    }
                } else {
                    let mut diagnostic = InterpreterDiagnostic::new(
                        &destination.location.source_file,
                        format!("Variable '{}' is not defined here", destination.lexeme()),
                    );
                    diagnostic.add_primary_label(&destination.location);
                    return Err(diagnostic.into());
                }
            }
            Expr::Call(call) => {
                self.resolve_expr(&mut call.callee)?;
                for arg in &mut call.arguments {
                    self.resolve_expr(arg)?
                }
                let TypeKind::Function(function_type) = call.callee.ty.kind() else {
                    return self.fail_fast(
                        &call.callee.location,
                        format!(
                            "Expected a function to call, but instead found type {}",
                            call.callee.ty
                        ),
                    );
                };
                if function_type.parameter_types.len() != call.arguments.len() {
                    return self.fail_fast(
                        &expr.location,
                        format!(
                            "Wrong number of arguments in call - expected: {}, actual {}",
                            function_type.parameter_types.len(),
                            call.arguments.len()
                        ),
                    );
                }
                for (parameter, argument) in
                    function_type.parameter_types.iter().zip(&call.arguments)
                {
                    if !self.type_checker.is_assignable_to(&argument.ty, parameter) {
                        return self.fail_fast(
                            &argument.location,
                            format!(
                                "Cannot coerce argument of type {} as parameter of type {} in function invocation",
                                argument.ty, parameter,
                            ),
                        );
                    }
                }
                expr.ty = function_type.return_type.clone();
            }
            Expr::Tuple(tuple) => {
                let mut component_types = Vec::<Type>::new();
                for component in &mut tuple.components {
                    self.resolve_expr(component)?;
                    component_types.push(component.ty.clone());
                }
                expr.ty = self.type_factory.tuple(component_types)
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
        self.scopes
            .iter_mut()
            .last()
            .expect("Scope Stack should not be empty")
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

    fn fail_fast<T, S: Into<String>>(&self, location: &Location, message: S) -> FelicoResult<T> {
        let mut diagnostic = InterpreterDiagnostic::new(&location.source_file, message.into());
        diagnostic.add_primary_label(location);
        Err(diagnostic.into())
    }
}

pub fn resolve_variables(
    ast: &mut AstNode<Program>,
    type_factory: &TypeFactory,
) -> FelicoResult<()> {
    ResolverPass::new(type_factory.clone()).resolve_program(ast)
}

#[cfg(test)]
mod tests {
    use crate::frontend::ast::print_ast::AstPrinter;
    use crate::frontend::parse::parser::Parser;
    use crate::frontend::resolve::resolver_pass::resolve_variables;
    use crate::infra::diagnostic::unwrap_diagnostic_to_string;
    use crate::interpret::core_definitions::TypeFactory;
    use expect_test::{expect, Expect};

    fn test_resolve_program(name: &str, input: &str, expected: Expect) {
        let type_factory = &TypeFactory::new();
        let parser = Parser::new_in_memory(name, input, type_factory).unwrap();
        let mut program = parser.parse_program().unwrap();
        resolve_variables(&mut program, type_factory).unwrap();
        let printed_ast = AstPrinter::new()
            .with_locations(false)
            .with_types(true)
            .print(&program)
            .unwrap();

        expected.assert_eq(&printed_ast);
    }

    macro_rules! test_program {
    ( $($label:ident: $input:expr => $expect:expr;)+ ) => {
        $(
            #[test]
            fn $label() {
                test_resolve_program(stringify!($label), $input, $expect);
            }
        )*
        }
    }

    test_program!(
                let_explicit_type: "let a: bool = true;" => expect![[r#"
                    Program
                    └── Let ''a' (Identifier)': ❬bool❭
                        └── Bool(true): ❬bool❭
                "#]];
                let_inferred_type: "let a = 3;" => expect![[r#"
                    Program
                    └── Let ''a' (Identifier)': ❬f64❭
                        └── F64(3.0): ❬f64❭
                "#]];
                let_inferred_type_from_binary_expression: "let a = 1 + 2;" => expect![[r#"
                    Program
                    └── Let ''a' (Identifier)': ❬f64❭
                        └── +: ❬f64❭
                            ├── F64(1.0): ❬f64❭
                            └── F64(2.0): ❬f64❭
                "#]];
                let_inferred_type_from_unary_expression: "let a = -1;" => expect![[r#"
                    Program
                    └── Let ''a' (Identifier)': ❬f64❭
                        └── -: ❬f64❭
                            └── F64(1.0): ❬f64❭
                "#]];
                let_inferred_type_from_variable: "let a = 1;let b = a;" => expect![[r#"
                    Program
                    ├── Let ''a' (Identifier)': ❬f64❭
                    │   └── F64(1.0): ❬f64❭
                    └── Let ''b' (Identifier)': ❬f64❭
                        └── Read 'a': ❬f64❭
                "#]];
                assign_type: "let a = 1;a = 3;" => expect![[r#"
                    Program
                    ├── Let ''a' (Identifier)': ❬f64❭
                    │   └── F64(1.0): ❬f64❭
                    └── 'a' (Identifier) = : ❬f64❭
                        └── F64(3.0): ❬f64❭
                "#]];
                tuple_empty: "();" => expect![[r#"
                    Program
                    └── Tuple: ❬()❭
                "#]];
                tuple_pair: "(3, true);" => expect![[r#"
                    Program
                    └── Tuple: ❬(❬f64❭, ❬bool❭)❭
                        ├── F64(3.0): ❬f64❭
                        └── Bool(true): ❬bool❭
                "#]];
                call_type_native: "sqrt(3);" => expect![[r#"
                    Program
                    └── Call: ❬f64❭
                        ├── Read 'sqrt': ❬Fn(❬f64❭)❬f64❭❭
                        └── F64(3.0): ❬f64❭
                "#]];
                function_simple: "fun x(a: bool, b: i64) -> f64 {} let a = x;" => expect![[r#"
                    Program
                    ├── Declare fun 'x(a, b)': ❬Fn(❬bool❭, ❬i64❭)❬f64❭❭
                    │   ├── Param a
                    │   │   └── Read 'bool'
                    │   ├── Param b
                    │   │   └── Read 'i64'
                    │   └── Return type: Read 'f64'
                    └── Let ''a' (Identifier)': ❬Fn(❬bool❭, ❬i64❭)❬f64❭❭
                        └── Read 'x': ❬Fn(❬bool❭, ❬i64❭)❬f64❭❭
                "#]];
                function_arg_type: "fun x(a: bool, b: i64) -> f64 {
                    a;
                    b;
                }" => expect![[r#"
                    Program
                    └── Declare fun 'x(a, b)': ❬Fn(❬bool❭, ❬i64❭)❬f64❭❭
                        ├── Param a
                        │   └── Read 'bool'
                        ├── Param b
                        │   └── Read 'i64'
                        ├── Return type: Read 'f64'
                        ├── Read 'a': ❬bool❭
                        └── Read 'b': ❬i64❭
                "#]];
               program_struct_simple: "
               struct Foo {
                    bar: bool,
                    baz: f64
                }
               " => expect![[r#"
                   Program
                   └── Struct 'Foo': ❬Foo❭
                       ├── Field bar: ❬bool❭
                       │   └── Read 'bool': ❬Type❭
                       └── Field baz: ❬f64❭
                           └── Read 'f64': ❬Type❭
               "#]];
                program_struct_empty: "
               struct Empty {}
               " => expect![[r#"
                   Program
                   └── Struct 'Empty': ❬Empty❭
               "#]];

    );
    fn test_resolve_program_error(name: &str, input: &str, expected: Expect) {
        let type_factory = &TypeFactory::new();
        let parser = Parser::new_in_memory(name, input, type_factory).unwrap();
        let mut ast = parser.parse_program().unwrap();
        let result = resolve_variables(&mut ast, type_factory);
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
        initialize_variable_with_wrong_type: r#"
        fun f() {
            let x: bool = 3;
         }"# => expect![[r#"
             × Expression value of type ❬f64❭ cannot be assigned to variable 'x' declared to be type ❬bool❭
                ╭─[initialize_variable_with_wrong_type:3:13]
              2 │         fun f() {
              3 │             let x: bool = 3;
                ·             ────────────────
              4 │          }
                ╰────"#]];
        call_a_non_function: r#"
        true();
        "# => expect![[r#"
            × Expected a function to call, but instead found type ❬bool❭
               ╭─[call_a_non_function:2:9]
             1 │ 
             2 │         true();
               ·         ────
             3 │         
               ╰────"#]];
        call_wrong_number_of_arguments: r#"
        sqrt(1,2);
        "# => expect![[r#"
            × Wrong number of arguments in call - expected: 1, actual 2
               ╭─[call_wrong_number_of_arguments:2:13]
             1 │ 
             2 │         sqrt(1,2);
               ·             ──────
             3 │         
               ╰────"#]];

        call_with_wrong_argument: r#"
        sqrt(true);
        "# => expect![[r#"
            × Cannot coerce argument of type ❬bool❭ as parameter of type ❬f64❭ in function invocation
               ╭─[call_with_wrong_argument:2:14]
             1 │ 
             2 │         sqrt(true);
               ·              ────
             3 │         
               ╰────"#]];
    );
}
