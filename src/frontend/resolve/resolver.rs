use crate::frontend::ast::expr::{
    AssignExpr, BinaryExpr, CallExpr, Expr, GetExpr, LiteralExpr, SetExpr, TupleExpr, UnaryExpr,
    VarUse,
};
use crate::frontend::ast::node::AstNode;
use crate::frontend::ast::program::Program;
use crate::frontend::ast::stmt::Stmt::Let;
use crate::frontend::ast::stmt::{
    BlockStmt, FunStmt, IfStmt, LetStmt, Stmt, StructStmt, WhileStmt,
};
use crate::frontend::ast::types::{StructField, Type, TypeKind};
use crate::frontend::lex::token::Token;
use crate::frontend::resolve::module_manifest::{ModuleEntry, ModuleManifest};
use crate::frontend::resolve::type_checker::TypeChecker;
use crate::infra::diagnostic::InterpreterDiagnostic;
use crate::infra::location::Location;
use crate::infra::result::{bail, FelicoResult};
use crate::infra::shared_string::{Name, SharedString};
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

pub struct Resolver {
    scopes: Vec<HashMap<String, Symbol>>,
    type_factory: TypeFactory,
    type_checker: TypeChecker,
}

// Ast information extract during resolution to make separate borrows
struct CommonAstInfo<'a> {
    location: &'a Location,
    ty: &'a mut Type,
}
impl<'a> CommonAstInfo<'a> {
    fn new(location: &'a Location, ty: &'a mut Type) -> Self {
        Self { location, ty }
    }
}

impl Resolver {
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
        Resolver {
            scopes: vec![global_scope, Default::default()],
            type_factory,
            type_checker: TypeChecker::new(),
        }
    }
    pub fn resolve_program(&mut self, program: &mut AstNode<Program>) -> FelicoResult<()> {
        self.resolve_stmts(&mut program.data.stmts)
    }

    pub fn get_module_manifest(&self) -> FelicoResult<ModuleManifest> {
        let module_entries: HashMap<Name, ModuleEntry> = self.scopes[1]
            .iter()
            .map(|(name, symbol)| {
                let name = SharedString::from(name);
                (
                    name.clone(),
                    ModuleEntry {
                        name,
                        ty: symbol.ty.clone(),
                    },
                )
            })
            .collect();
        Ok(ModuleManifest { module_entries })
    }

    fn resolve_stmts(&mut self, stmts: &mut Vec<AstNode<Stmt>>) -> FelicoResult<()> {
        for stmt in stmts {
            self.resolve_stmt(stmt)?;
        }
        Ok(())
    }

    fn resolve_stmt(&mut self, stmt: &mut AstNode<Stmt>) -> FelicoResult<()> {
        let mut ast_info = CommonAstInfo::new(&stmt.location, &mut stmt.ty);
        match stmt.data.deref_mut() {
            Let(let_stmt) => {
                self.resolve_let_stmt(let_stmt, &mut ast_info)?;
            }
            Stmt::Return(return_stmt) => {
                self.resolve_expr(&mut return_stmt.expression)?;
            }
            Stmt::Expression(expr_stmt) => {
                self.resolve_expr(&mut expr_stmt.expression)?;
            }
            Stmt::Struct(struct_stmt) => {
                self.resolve_struct_stmt(struct_stmt, &mut ast_info)?;
            }
            Stmt::Fun(fun_stmt) => {
                self.resolve_fun_stmt(fun_stmt, &mut ast_info)?;
            }
            Stmt::Block(block) => {
                self.resolve_block_stmt(block)?;
            }
            Stmt::If(if_stmt) => {
                self.resolve_if_stmt(if_stmt)?;
            }
            Stmt::While(while_stmt) => {
                self.resolve_while_stmt(while_stmt)?;
            }
        }
        Ok(())
    }

    fn resolve_while_stmt(&mut self, while_stmt: &mut WhileStmt) -> FelicoResult<()> {
        self.resolve_expr(&mut while_stmt.condition)?;
        self.resolve_stmt(&mut while_stmt.body_stmt)?;
        Ok(())
    }

    fn resolve_if_stmt(&mut self, if_stmt: &mut IfStmt) -> FelicoResult<()> {
        self.resolve_expr(&mut if_stmt.condition)?;
        self.resolve_stmt(&mut if_stmt.then_stmt)?;
        if let Some(stmt) = &mut if_stmt.else_stmt {
            self.resolve_stmt(stmt)?;
        }
        Ok(())
    }

    fn resolve_block_stmt(&mut self, block: &mut BlockStmt) -> FelicoResult<()> {
        self.scopes.push(Default::default());
        self.resolve_stmts(&mut block.stmts)?;
        self.scopes.pop();
        Ok(())
    }

    fn resolve_fun_stmt(
        &mut self,
        fun_stmt: &mut FunStmt,
        ast_info: &mut CommonAstInfo,
    ) -> FelicoResult<()> {
        let type_factory = self.type_factory.clone();
        let name = fun_stmt.name.lexeme();
        let return_type = self.resolve_type(&fun_stmt.return_type)?;
        let parameter_types: Vec<Type> = fun_stmt
            .parameters
            .iter()
            .map(|parameter| self.resolve_type(&parameter.type_expression))
            .collect::<FelicoResult<Vec<_>>>()?;
        let function_type = type_factory.function(parameter_types, return_type);
        self.add_symbol_to_scope(
            name.to_string(),
            Symbol {
                declaration_site: fun_stmt.name.location.clone(),
                is_defined: true,
                ty: function_type.clone(),
                value: None,
            },
        )?;
        *ast_info.ty = function_type;
        self.scopes.push(Default::default());
        for parameter in &fun_stmt.parameters {
            let ty = self.resolve_type(&parameter.type_expression)?.clone();
            self.add_symbol_to_scope(
                parameter.name.lexeme().to_string(),
                Symbol {
                    declaration_site: parameter.name.location.clone(),
                    is_defined: true,
                    ty,
                    value: None,
                },
            )?;
        }
        self.resolve_stmt(&mut fun_stmt.body)?;
        self.scopes.pop();
        Ok(())
    }

    fn resolve_struct_stmt(
        &mut self,
        struct_stmt: &mut StructStmt,
        ast_info: &mut CommonAstInfo,
    ) -> FelicoResult<()> {
        let type_factory = self.type_factory.clone();
        let mut fields = HashMap::new();
        for field in &mut struct_stmt.fields {
            self.resolve_expr(&mut field.data.type_expression)?;
            field.ty = self.resolve_type(&field.data.type_expression)?;
            let name = SharedString::from(field.data.name.lexeme());
            fields.insert(name.clone(), StructField::new(&field.data.name, &field.ty));
        }
        *ast_info.ty = type_factory.make_struct(&struct_stmt.name, fields);
        self.add_symbol_to_scope(
            struct_stmt.name.lexeme().into(),
            Symbol {
                declaration_site: struct_stmt.name.location.clone(),
                is_defined: true,
                ty: ast_info.ty.clone(),
                value: None,
            },
        )?;
        Ok(())
    }

    fn resolve_let_stmt(
        &mut self,
        let_stmt: &mut LetStmt,
        stmt: &mut CommonAstInfo,
    ) -> FelicoResult<()> {
        let name = let_stmt.name.lexeme();
        self.add_symbol_to_scope(
            name.to_string(),
            Symbol {
                declaration_site: let_stmt.name.location.clone(),
                is_defined: false,
                ty: self.type_factory.unknown(),
                value: None,
            },
        )?;
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
        *stmt.ty = variable_type.clone();
        let symbol = self.current_scope().get_mut(name).unwrap();
        symbol.is_defined = true;
        symbol.ty = variable_type;
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
        let mut ast_info = CommonAstInfo::new(&expr.location, &mut expr.ty);
        match expr.data.deref_mut() {
            Expr::Unary(unary) => {
                self.resolve_unary_expr(unary, &mut ast_info)?;
            }
            Expr::Binary(binary) => {
                self.resolve_binary_expr(binary, &mut ast_info)?;
            }
            Expr::Literal(literal) => {
                self.resolve_literal_expr(literal, &mut ast_info)?;
            }
            Expr::Assign(assign) => {
                self.resolve_assign_expr(assign, &mut ast_info)?;
            }
            Expr::Call(call) => {
                self.resolve_call_expr(call, &mut ast_info)?;
            }
            Expr::Variable(var_use) => self.resolve_var_use_expr(var_use, &mut ast_info)?,
            Expr::Tuple(tuple) => {
                self.resolve_tuple_expr(tuple, &mut ast_info)?;
            }
            Expr::Get(get) => {
                self.resolve_get_expr(get)?;
            }
            Expr::Set(set) => {
                self.resolve_set_expr(set)?;
            }
        }
        Ok(())
    }

    fn resolve_set_expr(&mut self, set: &mut SetExpr) -> FelicoResult<()> {
        self.resolve_expr(&mut set.value)?;
        self.resolve_expr(&mut set.object)?;
        Ok(())
    }

    fn resolve_get_expr(&mut self, get: &mut GetExpr) -> FelicoResult<()> {
        self.resolve_expr(&mut get.object)
    }

    fn resolve_tuple_expr(
        &mut self,
        tuple: &mut TupleExpr,
        ast_info: &mut CommonAstInfo,
    ) -> FelicoResult<()> {
        let mut component_types = Vec::<Type>::new();
        for component in &mut tuple.components {
            self.resolve_expr(component)?;
            component_types.push(component.ty.clone());
        }
        *ast_info.ty = self.type_factory.tuple(component_types);
        Ok(())
    }

    fn resolve_call_expr(
        &mut self,
        call: &mut CallExpr,
        ast_info: &mut CommonAstInfo,
    ) -> FelicoResult<()> {
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
                &ast_info.location,
                format!(
                    "Wrong number of arguments in call - expected: {}, actual {}",
                    function_type.parameter_types.len(),
                    call.arguments.len()
                ),
            );
        }
        for (parameter, argument) in function_type.parameter_types.iter().zip(&call.arguments) {
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
        *ast_info.ty = function_type.return_type.clone();
        Ok(())
    }

    fn resolve_assign_expr(
        &mut self,
        assign: &mut AssignExpr,
        ast_info: &mut CommonAstInfo,
    ) -> FelicoResult<()> {
        let destination = &assign.destination;
        self.resolve_expr(&mut assign.value)?;
        let distance_and_symbol = self.get_definition_distance_and_symbol(destination);
        if let Some((distance, symbol)) = distance_and_symbol {
            assign.distance = distance;
            let destination_type = &symbol.ty;
            *ast_info.ty = symbol.ty.clone();

            if !destination_type.is_unknown() {
                let expression_type = &assign.value.ty;
                if !self
                    .type_checker
                    .is_assignable_to(expression_type, destination_type)
                {
                    let mut diagnostic = InterpreterDiagnostic::new(&destination.location.source_file, format!("Expression value of type {} cannot be assigned to variable '{}' of type {}", expression_type, assign.destination.lexeme(), destination_type));
                    diagnostic.add_primary_label(&ast_info.location);
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
        Ok(())
    }

    fn resolve_var_use_expr(
        &mut self,
        var_use: &mut VarUse,
        ast_info: &mut CommonAstInfo,
    ) -> FelicoResult<()> {
        let distance_and_symbol = self.get_definition_distance_and_symbol(&var_use.variable);
        if let Some((distance, symbol)) = distance_and_symbol {
            var_use.distance = distance;
            *ast_info.ty = symbol.ty.clone();
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
        Ok(())
    }

    fn resolve_literal_expr(
        &mut self,
        literal: &mut LiteralExpr,
        ast_info: &mut CommonAstInfo,
    ) -> FelicoResult<()> {
        *ast_info.ty = match literal {
            LiteralExpr::Str(_) => self.type_factory.str(),
            LiteralExpr::F64(_) => self.type_factory.f64(),
            LiteralExpr::I64(_) => self.type_factory.i64(),
            LiteralExpr::Bool(_) => self.type_factory.bool(),
            LiteralExpr::Unit => self.type_factory.unit(),
        };
        Ok(())
    }

    fn resolve_binary_expr(
        &mut self,
        binary: &mut BinaryExpr,
        ast_info: &mut CommonAstInfo,
    ) -> FelicoResult<()> {
        self.resolve_expr(&mut binary.left)?;
        self.resolve_expr(&mut binary.right)?;
        if binary.left.ty == binary.right.ty {
            *ast_info.ty = binary.left.ty.clone();
        }
        Ok(())
    }

    fn resolve_unary_expr(
        &mut self,
        unary: &mut UnaryExpr,
        ast_info: &mut CommonAstInfo,
    ) -> FelicoResult<()> {
        self.resolve_expr(&mut unary.right)?;
        *ast_info.ty = unary.right.ty.clone();
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
                return if entry.is_defined {
                    Some((distance, entry))
                } else {
                    None
                };
            }
        }
        None
    }

    fn fail_fast<T, S: Into<String>>(&self, location: &Location, message: S) -> FelicoResult<T> {
        let mut diagnostic = InterpreterDiagnostic::new(&location.source_file, message.into());
        diagnostic.add_primary_label(location);
        Err(diagnostic.into())
    }

    fn add_symbol_to_scope(&mut self, name: String, symbol: Symbol) -> FelicoResult<()> {
        match self.current_scope().entry(name.clone()) {
            Entry::Occupied(value) => {
                let mut diagnostic = InterpreterDiagnostic::new(
                    &symbol.declaration_site.source_file,
                    format!("The name '{}' already declared", name),
                );
                diagnostic.add_primary_label(&symbol.declaration_site);
                diagnostic.add_label(&value.get().declaration_site, "is already declared here");
                return Err(diagnostic.into());
            }
            Entry::Vacant(slot) => {
                slot.insert(symbol);
            }
        }
        Ok(())
    }
}

pub fn resolve_variables(
    ast: &mut AstNode<Program>,
    type_factory: &TypeFactory,
) -> FelicoResult<()> {
    Resolver::new(type_factory.clone()).resolve_program(ast)
}

#[cfg(test)]
mod tests {
    use crate::frontend::ast::print_ast::AstPrinter;
    use crate::frontend::parse::parser::Parser;
    use crate::frontend::resolve::resolver::{resolve_variables, Resolver};
    use crate::infra::diagnostic::unwrap_diagnostic_to_string;
    use crate::interpret::core_definitions::TypeFactory;
    use expect_test::{expect, Expect};

    fn test_resolve_program(
        name: &str,
        input: &str,
        expected_ast: Expect,
        expected_manifest: Expect,
    ) {
        let type_factory = &TypeFactory::new();
        let parser = Parser::new_in_memory(name, input, type_factory).unwrap();
        let mut program = parser.parse_program().unwrap();
        let mut resolver = Resolver::new(type_factory.clone());
        resolver.resolve_program(&mut program).unwrap();
        let printed_ast = AstPrinter::new()
            .with_locations(false)
            .with_types(true)
            .print(&program)
            .unwrap();

        expected_ast.assert_eq(&printed_ast);
        expected_manifest.assert_eq(&resolver.get_module_manifest().unwrap().as_pretty_string());
    }

    macro_rules! test_program {
    ( $($label:ident: $input:expr => $expected_ast:expr,$expected_manifest:expr;)+ ) => {
        $(
            #[test]
            fn $label() {
                test_resolve_program(stringify!($label), $input, $expected_ast, $expected_manifest);
            }
        )*
        }
    }

    test_program!(
        let_explicit_type: "let a: bool = true;" => expect![[r#"
            Program
            └── Let ''a' (Identifier)': ❬bool❭
                └── Bool(true): ❬bool❭
        "#]],expect![[r#"
            Module
              a: ❬bool❭
        "#]];

        let_inferred_type: "let a = 3;" => expect![[r#"
                Program
                └── Let ''a' (Identifier)': ❬f64❭
                    └── F64(3.0): ❬f64❭
            "#]],expect![[r#"
                Module
                  a: ❬f64❭
            "#]];
        let_inferred_type_from_binary_expression: "let a = 1 + 2;" => expect![[r#"
                Program
                └── Let ''a' (Identifier)': ❬f64❭
                    └── +: ❬f64❭
                        ├── F64(1.0): ❬f64❭
                        └── F64(2.0): ❬f64❭
            "#]],expect![[r#"
                Module
                  a: ❬f64❭
            "#]];
        let_inferred_type_from_unary_expression: "let a = -1;" => expect![[r#"
                Program
                └── Let ''a' (Identifier)': ❬f64❭
                    └── -: ❬f64❭
                        └── F64(1.0): ❬f64❭
            "#]],expect![[r#"
                Module
                  a: ❬f64❭
            "#]];
        let_inferred_type_from_variable: "let a = 1;let b = a;" => expect![[r#"
            Program
            ├── Let ''a' (Identifier)': ❬f64❭
            │   └── F64(1.0): ❬f64❭
            └── Let ''b' (Identifier)': ❬f64❭
                └── Read 'a': ❬f64❭
        "#]],expect![[r#"
            Module
              a: ❬f64❭
              b: ❬f64❭
        "#]];
        assign_type: "let a = 1;a = 3;" => expect![[r#"
            Program
            ├── Let ''a' (Identifier)': ❬f64❭
            │   └── F64(1.0): ❬f64❭
            └── 'a' (Identifier) = : ❬f64❭
                └── F64(3.0): ❬f64❭
        "#]],expect![[r#"
            Module
              a: ❬f64❭
        "#]];
        tuple_empty: "();" => expect![[r#"
                Program
                └── Tuple: ❬()❭
            "#]],expect![[r#"
                Module
            "#]];
        tuple_pair: "(3, true);" => expect![[r#"
                Program
                └── Tuple: ❬(❬f64❭, ❬bool❭)❭
                    ├── F64(3.0): ❬f64❭
                    └── Bool(true): ❬bool❭
            "#]],expect![[r#"
                Module
            "#]];
        call_type_native: "sqrt(3);" => expect![[r#"
            Program
            └── Call: ❬f64❭
                ├── Read 'sqrt': ❬Fn(❬f64❭) -> ❬f64❭❭
                └── F64(3.0): ❬f64❭
        "#]],expect![[r#"
                Module
            "#]];
        function_simple: "fun x(a: bool, b: i64) -> f64 {} let a = x;" => expect![[r#"
            Program
            ├── Declare fun 'x(a, b)': ❬Fn(❬bool❭, ❬i64❭) -> ❬f64❭❭
            │   ├── Param a
            │   │   └── Read 'bool'
            │   ├── Param b
            │   │   └── Read 'i64'
            │   └── Return type: Read 'f64'
            └── Let ''a' (Identifier)': ❬Fn(❬bool❭, ❬i64❭) -> ❬f64❭❭
                └── Read 'x': ❬Fn(❬bool❭, ❬i64❭) -> ❬f64❭❭
        "#]],expect![[r#"
            Module
              a: ❬Fn(❬bool❭, ❬i64❭) -> ❬f64❭❭
              x: ❬Fn(❬bool❭, ❬i64❭) -> ❬f64❭❭
        "#]];
        function_arg_type: "fun x(a: bool, b: i64) -> f64 {
                a;
                b;
            }" => expect![[r#"
                Program
                └── Declare fun 'x(a, b)': ❬Fn(❬bool❭, ❬i64❭) -> ❬f64❭❭
                    ├── Param a
                    │   └── Read 'bool'
                    ├── Param b
                    │   └── Read 'i64'
                    ├── Return type: Read 'f64'
                    ├── Read 'a': ❬bool❭
                    └── Read 'b': ❬i64❭
            "#]],expect![[r#"
                Module
                  x: ❬Fn(❬bool❭, ❬i64❭) -> ❬f64❭❭
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
           "#]],expect![[r#"
               Module
                 Foo: ❬Foo❭
                   baz: ❬f64❭
                   bar: ❬bool❭
           "#]];
        program_struct_empty: "
           struct Empty {}
           " => expect![[r#"
               Program
               └── Struct 'Empty': ❬Empty❭
           "#]],expect![[r#"
               Module
                 Empty: ❬Empty❭
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
        let_self_referential: "let a: bool = a;" => expect![[r#"
            × Variable 'a' is not defined here
               ╭─[let_self_referential:1:15]
             1 │ let a: bool = a;
               ·               ─
               ╰────"#]];
        let_self_referential_quasi: r#"
        let a = "outer";
        {
          let a = a;
        }
        "# => expect![[r#"
            × Variable 'a' is not defined here
               ╭─[let_self_referential_quasi:4:19]
             3 │         {
             4 │           let a = a;
               ·                   ─
             5 │         }
               ╰────"#]];
        double_declaration: "let x = 0;\ndebug_print(x);\nlet x = true;" => expect![[r#"
            × The name 'x' already declared
               ╭─[double_declaration:3:5]
             1 │ let x = 0;
               ·     ┬
               ·     ╰── is already declared here
             2 │ debug_print(x);
             3 │ let x = true;
               ·     ─
               ╰────"#]];
        double_declaration_with_function: "let x = 0;\nfun x() {}" => expect![[r#"
            × The name 'x' already declared
               ╭─[double_declaration_with_function:2:5]
             1 │ let x = 0;
               ·     ┬
               ·     ╰── is already declared here
             2 │ fun x() {}
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
