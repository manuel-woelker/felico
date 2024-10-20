use crate::frontend::ast::expr::{
    AssignExpr, BinaryExpr, BlockExpr, CallExpr, CreateStructExpr, Expr, GetExpr, IfExpr,
    LiteralExpr, ReturnExpr, SetExpr, UnaryExpr, VarUse,
};
use crate::frontend::ast::module::Module;
use crate::frontend::ast::node::AstNode;
use crate::frontend::ast::qualified_name::QualifiedName;
use crate::frontend::ast::stmt::Stmt::Let;
use crate::frontend::ast::stmt::{
    FunStmt, ImplStmt, LetStmt, Stmt, StructStmt, TraitStmt, WhileStmt,
};
use crate::frontend::lex::token::Token;
use crate::frontend::resolve::module_manifest::{ModuleEntry, ModuleManifest};
use crate::frontend::resolve::type_checker::TypeChecker;
use crate::infra::diagnostic::InterpreterDiagnostic;
use crate::infra::full_name::FullName;
use crate::infra::result::{bail, FelicoError, FelicoReport, FelicoResult};
use crate::infra::shared_string::{Name, SharedString};
use crate::infra::source_span::SourceSpan;
use crate::interpret::core_definitions::get_core_definitions;
use crate::interpret::value::{InterpreterValue, ValueKind};
use crate::model::type_factory::TypeFactory;
use crate::model::types::{StructField, Type, TypeKind};
use crate::model::workspace::{Workspace, WorkspaceString};
use error_stack::Report;
use itertools::Itertools;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::ops::DerefMut;
use std::sync::Mutex;

pub type SymbolMap<'ws> = HashMap<SharedString<'ws>, Symbol<'ws>>;

#[derive(Debug)]
pub struct Symbol<'ws> {
    declaration_site: SourceSpan<'ws>,
    is_defined: bool,
    // Type of the symbol or expression
    ty: Type<'ws>,
    // Value of the expression
    value: Option<InterpreterValue<'ws>>,
    symbol_map: Mutex<SymbolMap<'ws>>,
}

impl<'ws> Symbol<'ws> {
    pub fn new(
        declaration_site: SourceSpan<'ws>,
        is_defined: bool,
        ty: Type<'ws>,
        value: Option<InterpreterValue<'ws>>,
    ) -> Symbol<'ws> {
        Symbol {
            declaration_site,
            is_defined,
            ty,
            value,
            symbol_map: Mutex::new(SymbolMap::new()),
        }
    }

    pub fn define_value(declaration_site: &SourceSpan<'ws>, value: InterpreterValue<'ws>) -> Self {
        Self::new(declaration_site.clone(), true, value.ty, Some(value))
    }

    pub fn define(declaration_site: &SourceSpan<'ws>, ty: &Type<'ws>) -> Self {
        Self::new(declaration_site.clone(), true, *ty, None)
    }
    pub fn declare(declaration_site: &SourceSpan<'ws>, ty: &Type<'ws>) -> Self {
        Self::new(declaration_site.clone(), false, *ty, None)
    }
}

struct CurrentFunctionInfo<'ws> {
    declared_return_type: Type<'ws>,
    return_type_declaration_site: SourceSpan<'ws>,
}

pub struct LexicalScope<'ws> {
    symbols: HashMap<SharedString<'ws>, Symbol<'ws>>,
    current_function: Option<CurrentFunctionInfo<'ws>>,
    base_name: FullName<'ws>,
}

impl<'ws> LexicalScope<'ws> {
    fn new(base_name: FullName<'ws>) -> Self {
        Self {
            symbols: Default::default(),
            current_function: None,
            base_name,
        }
    }
    fn insert<S: Into<SharedString<'ws>>>(&mut self, name: S, symbol: Symbol<'ws>) {
        self.symbols.insert(name.into(), symbol);
    }
    fn get(&self, name: &str) -> Option<&Symbol<'ws>> {
        self.symbols.get(name)
    }
    fn get_mut(&mut self, name: &str) -> Option<&mut Symbol<'ws>> {
        self.symbols.get_mut(name)
    }
    fn entry<S: Into<SharedString<'ws>>>(
        &mut self,
        name: S,
    ) -> Entry<SharedString<'ws>, Symbol<'ws>> {
        self.symbols.entry(name.into())
    }
}

pub struct Resolver<'ws> {
    scopes: Vec<LexicalScope<'ws>>,
    type_factory: TypeFactory<'ws>,
    type_checker: TypeChecker,
    diagnostics: Vec<InterpreterDiagnostic<'ws>>,
    module_name: FullName<'ws>,
}

// Ast information extract during resolution to make separate borrows
struct CommonAstInfo<'a, 'ws: 'a> {
    location: &'a SourceSpan<'ws>,
    ty: &'a mut Type<'ws>,
}
impl<'a, 'ws: 'a> CommonAstInfo<'a, 'ws> {
    fn new(location: &'a SourceSpan<'ws>, ty: &'a mut Type<'ws>) -> Self {
        Self { location, ty }
    }
}

impl<'ws> Resolver<'ws> {
    pub fn new(workspace: Workspace<'ws>) -> Resolver<'ws> {
        let mut global_scope: LexicalScope =
            LexicalScope::new(workspace.make_full_name("__global"));
        let location = SourceSpan {
            source_file: workspace.source_file_from_string("native", "native_code"),
            start_byte: 0,
            end_byte: 0,
        };
        for core_definition in
            get_core_definitions(workspace.value_factory(), workspace.type_factory())
        {
            global_scope.insert(
                core_definition.name,
                Symbol::define_value(&location, core_definition.value.clone()),
            );
        }
        Resolver {
            scopes: vec![global_scope],
            type_factory: workspace.type_factory(),
            type_checker: TypeChecker::new(),
            diagnostics: vec![],
            module_name: workspace.make_full_name("<undefined>"),
        }
    }

    pub fn diagnose(&mut self, diagnostic: InterpreterDiagnostic<'ws>) {
        self.diagnostics.push(diagnostic);
    }

    pub fn resolve_program(&mut self, program: &mut AstNode<'ws, Module<'ws>>) -> FelicoResult<()> {
        self.module_name = program.data.name;
        let module_scope = LexicalScope::new(self.module_name);
        self.scopes.push(module_scope);
        self.resolve_stmts(&mut program.data.stmts)?;
        if !self.diagnostics.is_empty() {
            let diagnostics = std::mem::take(&mut self.diagnostics);
            let report = diagnostics
                .into_iter()
                .map(|diagnostic| Report::from(FelicoError::from(diagnostic)))
                .reduce(|mut a, b| {
                    a.extend_one(b);
                    a
                })
                .unwrap();
            return Err(FelicoReport::new(report));
        }
        Ok(())
    }

    pub fn get_module_manifest(&self) -> FelicoResult<ModuleManifest<'ws>> {
        let module_entries: HashMap<Name, ModuleEntry> = self.scopes[1]
            .symbols
            .iter()
            .map(|(name, symbol)| {
                let name = SharedString::from(name);
                (
                    name,
                    ModuleEntry {
                        name,
                        type_signature: symbol.ty.to_string(),
                    },
                )
            })
            .collect();
        Ok(ModuleManifest {
            name: self.module_name,
            module_entries,
        })
    }

    fn resolve_stmts(&mut self, stmts: &mut Vec<AstNode<'ws, Stmt<'ws>>>) -> FelicoResult<()> {
        for stmt in stmts {
            self.resolve_stmt(stmt)?;
        }
        Ok(())
    }

    fn resolve_stmt(&mut self, stmt: &mut AstNode<'ws, Stmt<'ws>>) -> FelicoResult<()> {
        let mut ast_info = CommonAstInfo::new(&stmt.location, &mut stmt.ty);
        match stmt.data.deref_mut() {
            Let(let_stmt) => {
                self.resolve_let_stmt(let_stmt, &mut ast_info)?;
            }
            Stmt::Expression(expr_stmt) => {
                self.resolve_expr(&mut expr_stmt.expression)?;
            }
            Stmt::Struct(struct_stmt) => {
                self.resolve_struct_stmt(struct_stmt, &mut ast_info)?;
            }
            Stmt::Impl(impl_stmt) => {
                self.resolve_impl_stmt(impl_stmt, &mut ast_info)?;
            }
            Stmt::Trait(trait_stmt) => {
                self.resolve_trait_stmt(trait_stmt, &mut ast_info)?;
            }
            Stmt::Fun(fun_stmt) => {
                self.resolve_fun_stmt(fun_stmt, &mut ast_info)?;
            }
            Stmt::While(while_stmt) => {
                self.resolve_while_stmt(while_stmt)?;
            }
        }
        Ok(())
    }

    fn resolve_while_stmt(&mut self, while_stmt: &mut WhileStmt<'ws>) -> FelicoResult<()> {
        self.resolve_expr(&mut while_stmt.condition)?;
        self.resolve_stmt(&mut while_stmt.body_stmt)?;
        Ok(())
    }

    fn resolve_if_expr(
        &mut self,
        if_expr: &mut IfExpr<'ws>,
        ast_info: &mut CommonAstInfo<'_, 'ws>,
    ) -> FelicoResult<()> {
        self.resolve_expr(&mut if_expr.condition)?;
        let condition_type = &if_expr.condition.ty;
        if !condition_type.is_bool() {
            self.diagnose(InterpreterDiagnostic::new(&if_expr.condition.location, format!("Condition in if statement must evaluate to a boolean, but was of type {} instead", condition_type)))
        }
        self.resolve_expr(&mut if_expr.then_expr)?;
        let else_type = if let Some(else_expr) = &mut if_expr.else_expr {
            self.resolve_expr(else_expr)?;
            &else_expr.ty
        } else {
            &self.type_factory.unit()
        };
        let then_type = &if_expr.then_expr.ty;
        if else_type != then_type {
            self.diagnose(InterpreterDiagnostic::new(ast_info.location, format!("Then and else branch of if statement must evaluate to the same type, but then evaluates to {}, while else evaluates to {}", then_type, else_type)))
        }
        *ast_info.ty = if_expr.then_expr.ty;
        Ok(())
    }

    fn resolve_block_expr(
        &mut self,
        block: &mut BlockExpr<'ws>,
        ast_info: &mut CommonAstInfo<'_, 'ws>,
    ) -> FelicoResult<()> {
        let new_scope = LexicalScope::new(self.current_scope().base_name);
        self.scopes.push(new_scope);
        self.resolve_stmts(&mut block.stmts)?;
        self.resolve_expr(&mut block.result_expression)?;
        self.scopes.pop();
        *ast_info.ty = block.result_expression.ty;
        Ok(())
    }

    fn resolve_fun_stmt(
        &mut self,
        fun_stmt: &mut FunStmt<'ws>,
        ast_info: &mut CommonAstInfo<'_, 'ws>,
    ) -> FelicoResult<()> {
        let type_factory = self.type_factory;
        let name = fun_stmt.name.lexeme();
        let return_type = self.resolve_type(&fun_stmt.return_type)?;
        let parameter_types: Vec<Type> = fun_stmt
            .parameters
            .iter()
            .map(|parameter| self.resolve_type(&parameter.type_expression))
            .collect::<FelicoResult<Vec<_>>>()?;
        let function_type =
            type_factory.function(parameter_types, return_type, fun_stmt.name.location.clone());
        self.add_symbol_to_scope(
            name,
            Symbol::define(&fun_stmt.name.location, &function_type),
        )?;
        *ast_info.ty = function_type;
        let mut function_scope = LexicalScope::new(self.current_scope().base_name);
        function_scope.current_function = Some(CurrentFunctionInfo {
            return_type_declaration_site: fun_stmt.return_type.location.clone(),
            declared_return_type: return_type,
        });
        self.scopes.push(function_scope);
        for parameter in &fun_stmt.parameters {
            let ty = &self.resolve_type(&parameter.type_expression)?;
            self.add_symbol_to_scope(
                parameter.name.lexeme(),
                Symbol::define(&parameter.name.location, ty),
            )?;
        }
        self.resolve_expr(&mut fun_stmt.body)?;
        self.scopes.pop();
        Ok(())
    }

    fn resolve_struct_stmt(
        &mut self,
        struct_stmt: &mut StructStmt<'ws>,
        ast_info: &mut CommonAstInfo<'_, 'ws>,
    ) -> FelicoResult<()> {
        let type_factory = self.type_factory;
        let mut fields = HashMap::new();
        for field in &mut struct_stmt.fields {
            self.resolve_expr(&mut field.data.type_expression)?;
            field.ty = self.resolve_type(&field.data.type_expression)?;
            let name = SharedString::from(field.data.name.lexeme());
            fields.insert(name, StructField::new(&field.data.name, field.ty));
        }
        let ty =
            type_factory.make_struct(&struct_stmt.name, fields, struct_stmt.name.location.clone());
        *ast_info.ty = self.type_factory.ty();
        self.add_symbol_to_scope(
            struct_stmt.name.lexeme(),
            Symbol::define_value(
                &struct_stmt.name.location,
                InterpreterValue {
                    val: ValueKind::Type(ty),
                    ty,
                },
            ),
        )?;
        Ok(())
    }

    fn resolve_impl_stmt(
        &mut self,
        impl_stmt: &mut ImplStmt<'ws>,
        ast_info: &mut CommonAstInfo<'_, 'ws>,
    ) -> FelicoResult<()> {
        let new_scope = LexicalScope::new(self.current_scope().base_name);
        self.scopes.push(new_scope);
        for method in &mut impl_stmt.methods {
            self.resolve_fun_stmt(
                &mut method.data,
                &mut CommonAstInfo {
                    location: &method.location,
                    ty: &mut method.ty,
                },
            )?;
        }
        self.scopes.pop();
        let Some((_distance, symbol)) = self.get_definition_distance_and_symbol(&impl_stmt.name)
        else {
            bail!(
                "Could not find symbol entry for {}",
                impl_stmt.name.lexeme()
            );
        };

        for method in &mut impl_stmt.methods {
            symbol.symbol_map.lock().unwrap().insert(
                SharedString::from(method.data.name.lexeme()),
                Symbol::define(&method.location, &method.ty),
            );
        }
        *ast_info.ty = self.type_factory.unit();
        Ok(())
    }

    fn resolve_trait_stmt(
        &mut self,
        trait_stmt: &mut TraitStmt<'ws>,
        ast_info: &mut CommonAstInfo<'_, 'ws>,
    ) -> FelicoResult<()> {
        let type_factory = self.type_factory;
        *ast_info.ty = type_factory.make_trait(&trait_stmt.name, trait_stmt.name.location.clone());
        self.add_symbol_to_scope(
            trait_stmt.name.lexeme(),
            Symbol::define(&trait_stmt.name.location, ast_info.ty),
        )?;
        Ok(())
    }

    fn resolve_let_stmt(
        &mut self,
        let_stmt: &mut LetStmt<'ws>,
        ast_info: &mut CommonAstInfo<'_, 'ws>,
    ) -> FelicoResult<()> {
        let name = let_stmt.name.lexeme();
        self.add_symbol_to_scope(
            name,
            Symbol::declare(&let_stmt.name.location, &self.type_factory.unknown()),
        )?;
        self.resolve_expr(&mut let_stmt.expression)?;
        let expression_type = &let_stmt.expression.ty;
        let variable_type = if let Some(type_expr) = &let_stmt.type_expression {
            self.resolve_type(type_expr)?
        } else {
            let_stmt.expression.ty
        };
        if !self
            .type_checker
            .is_assignable_to(expression_type, &variable_type)
        {
            let diagnostic = InterpreterDiagnostic::new(ast_info.location, format!("Expression value of type {} cannot be assigned to variable '{}' declared to be type {}", expression_type, let_stmt.name.lexeme(), variable_type));
            self.diagnose(diagnostic);
            *ast_info.ty = self.type_factory.unresolved();
        } else {
            *ast_info.ty = variable_type;
        }
        let symbol = self.current_scope_mut().get_mut(name).unwrap();
        symbol.is_defined = true;
        symbol.ty = variable_type;
        Ok(())
    }

    fn resolve_type(&mut self, expr: &AstNode<'ws, Expr<'ws>>) -> FelicoResult<Type<'ws>> {
        // TODO: make bails into diagnostics
        let Expr::Variable(type_id) = &*expr.data else {
            bail!("Unsupported expression in type position: {:?}", expr);
        };
        let distance_and_symbol =
            self.get_definition_distance_and_symbol(&type_id.name.data.parts[0]);
        let Some((_distance, symbol)) = distance_and_symbol else {
            bail!("Unknown symbol: {}", type_id.name);
        };
        let Some(value) = &symbol.value else {
            dbg!(&self.current_scope().symbols);
            bail!("Unknown value for symbol: {}", type_id.name);
        };
        let ValueKind::Type(ty) = &value.val else {
            bail!("Type expression must be a type: {}", type_id.name);
        };
        Ok(*ty)
    }

    fn resolve_expr(&mut self, expr: &mut AstNode<'ws, Expr<'ws>>) -> FelicoResult<()> {
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
            Expr::Get(get) => {
                self.resolve_get_expr(get, &mut ast_info)?;
            }
            Expr::Set(set) => {
                self.resolve_set_expr(set, &mut ast_info)?;
            }
            Expr::Block(block) => {
                self.resolve_block_expr(block, &mut ast_info)?;
            }
            Expr::If(if_expr) => {
                self.resolve_if_expr(if_expr, &mut ast_info)?;
            }
            Expr::Return(return_expr) => {
                self.resolve_return_expr(return_expr, &mut ast_info)?;
            }
            Expr::CreateStruct(create_struct_expr) => {
                self.resolve_create_struct_expr(create_struct_expr, &mut ast_info)?;
            }
        }
        Ok(())
    }

    fn resolve_method_call_expr(
        &mut self,
        call: &mut CallExpr<'ws>,
        ast_info: &mut CommonAstInfo<'_, 'ws>,
    ) -> FelicoResult<bool> {
        if let Expr::Get(get_expr) = &mut *call.callee.data {
            self.resolve_expr(&mut get_expr.object)?;
            if self
                .get_struct_field(&get_expr.object.ty, get_expr.name.lexeme())?
                .is_none()
            {
                // Not a field, try to resolve function instead
                let distance_and_symbol = self.get_definition_distance_and_symbol(&get_expr.name);
                if let Some((distance, symbol)) = distance_and_symbol {
                    // Matching name found
                    if let TypeKind::Function(_fun) = symbol.ty.kind() {
                        // function in method call position, replace ast
                        let fun_node = AstNode::new(
                            Expr::Variable(VarUse {
                                name: AstNode::new(
                                    QualifiedName {
                                        parts: vec![get_expr.name.clone()],
                                    },
                                    get_expr.name.location.clone(),
                                    symbol.ty,
                                ),
                                distance,
                            }),
                            get_expr.name.location.clone(),
                            symbol.ty,
                        );
                        let first_argument = get_expr.object.clone();
                        call.callee = fun_node;
                        call.arguments.insert(0, first_argument);
                        self.resolve_call_expr(call, ast_info)?;
                        return Ok(true);
                    }
                }
            }
        }
        Ok(false)
    }

    fn resolve_set_expr(
        &mut self,
        set: &mut SetExpr<'ws>,
        ast_info: &mut CommonAstInfo<'_, 'ws>,
    ) -> FelicoResult<()> {
        self.resolve_expr(&mut set.value)?;
        self.resolve_expr(&mut set.object)?;
        let Some(field) = self.get_struct_field(&set.object.ty, set.name.lexeme())? else {
            let diagnostic = InterpreterDiagnostic::new(
                ast_info.location,
                format!(
                    "Type {} has no field '{}'",
                    set.object.ty,
                    set.name.lexeme()
                ),
            );
            self.diagnose(diagnostic);
            return Ok(());
        };
        if !self.type_checker.is_assignable_to(&set.value.ty, &field.ty) {
            let diagnostic = InterpreterDiagnostic::new(
                ast_info.location,
                format!(
                    "Expression value of type {} cannot be assigned to field '{}.{}' of type {}",
                    set.value.ty,
                    set.object.ty,
                    set.name.lexeme(),
                    field.ty
                ),
            );
            self.diagnose(diagnostic);
        }
        *ast_info.ty = field.ty;
        Ok(())
    }

    fn resolve_get_expr(
        &mut self,
        get: &mut GetExpr<'ws>,
        ast_info: &mut CommonAstInfo<'_, 'ws>,
    ) -> FelicoResult<()> {
        self.resolve_expr(&mut get.object)?;
        let Some(field) = self.get_struct_field(&get.object.ty, get.name.lexeme())? else {
            let diagnostic = InterpreterDiagnostic::new(
                ast_info.location,
                format!(
                    "Type {} has no field '{}'",
                    get.object.ty,
                    get.name.lexeme()
                ),
            );
            self.diagnose(diagnostic);
            return Ok(());
        };
        *ast_info.ty = field.ty;
        Ok(())
    }

    fn get_struct_field<'b>(
        &mut self,
        ty: &'b Type<'ws>,
        field_name: &str,
        //_ast_info: &mut CommonAstInfo,
    ) -> FelicoResult<Option<&'b StructField<'ws>>> {
        let TypeKind::Struct(struct_type) = ty.kind() else {
            /*            let diagnostic = InterpreterDiagnostic::new(ast_info.location,
                                                        format!("Using the dot operator to access a field, but the type of the object is not a struct but {}", ty));
            self.diagnose(diagnostic);*/
            return Ok(None);
        };
        let Some(field) = struct_type.fields.get(field_name) else {
            /*            let diagnostic = InterpreterDiagnostic::new(
                ast_info.location,
                format!("Struct {} has no field '{}'", ty, field_name),
            );
            self.diagnose(diagnostic);*/
            return Ok(None);
        };
        Ok(Some(field))
    }

    fn resolve_call_expr(
        &mut self,
        call: &mut CallExpr<'ws>,
        ast_info: &mut CommonAstInfo<'_, 'ws>,
    ) -> FelicoResult<()> {
        if self.resolve_method_call_expr(call, ast_info)? {
            return Ok(());
        }

        self.resolve_expr(&mut call.callee)?;

        for arg in &mut call.arguments {
            self.resolve_expr(arg)?
        }
        let TypeKind::Function(function_type) = call.callee.ty.kind() else {
            self.diagnose(InterpreterDiagnostic::new(
                &call.callee.location,
                format!(
                    "Expected a function to call, but instead found type {}",
                    call.callee.ty
                ),
            ));
            *ast_info.ty = self.type_factory.unresolved();
            return Ok(());
        };
        *ast_info.ty = function_type.return_type;
        let mut diagnostics = vec![];
        let mut diagnose = |location: &SourceSpan<'ws>, message: String| {
            let mut diagnostic = InterpreterDiagnostic::new(location, message);
            let declaration_site = call.callee.ty.declaration_site();
            if !declaration_site.is_ephemeral() {
                diagnostic.add_label(declaration_site, "Function declared here".to_string());
            }
            diagnostics.push(diagnostic);
        };

        if function_type.parameter_types.len() != call.arguments.len() {
            diagnose(
                ast_info.location,
                format!(
                    "Wrong number of arguments in call - expected: {}, actual {}",
                    function_type.parameter_types.len(),
                    call.arguments.len()
                ),
            );
        }
        for (parameter, argument) in function_type.parameter_types.iter().zip(&call.arguments) {
            if !self.type_checker.is_assignable_to(&argument.ty, parameter) {
                diagnose(
                    &argument.location,
                    format!(
                        "Cannot coerce argument of type {} as parameter of type {} in function invocation",
                        argument.ty, parameter,
                    ),
                );
            }
        }
        self.diagnostics.append(&mut diagnostics);
        Ok(())
    }

    fn resolve_create_struct_expr(
        &mut self,
        create_struct_expr: &mut CreateStructExpr<'ws>,
        ast_info: &mut CommonAstInfo<'_, 'ws>,
    ) -> FelicoResult<()> {
        let type_expression = &mut create_struct_expr.type_expression;
        self.resolve_expr(type_expression)?;
        // TODO: should return a type constructor, not the type itself
        let TypeKind::Struct(struct_type) = type_expression.ty.kind() else {
            self.diagnose(InterpreterDiagnostic::new(
                &type_expression.location,
                format!(
                    "Expected a struct to create, but instead found type {}",
                    type_expression.ty
                ),
            ));
            *ast_info.ty = self.type_factory.unresolved();
            return Ok(());
        };
        *ast_info.ty = type_expression.ty;
        let mut diagnostics = vec![];
        let mut diagnose = |location: &SourceSpan<'ws>, message: String| {
            let mut diagnostic = InterpreterDiagnostic::new(location, message);
            let declaration_site = type_expression.ty.declaration_site();
            if !declaration_site.is_ephemeral() {
                diagnostic.add_label(declaration_site, "Struct declared here".to_string());
            }
            diagnostics.push(diagnostic);
        };
        let mut seen_fields: HashMap<&str, SourceSpan> = Default::default();
        for field_initializer in &mut create_struct_expr.field_initializers {
            self.resolve_expr(&mut field_initializer.expression)?;
            let Some(field) = struct_type
                .fields
                .get(field_initializer.field_name.lexeme())
            else {
                diagnose(
                    &field_initializer.field_name.location,
                    format!(
                        "Struct {} does not have a field named '{}'",
                        type_expression.ty,
                        field_initializer.field_name.lexeme(),
                    ),
                );
                continue;
            };
            if !self
                .type_checker
                .is_assignable_to(&field_initializer.expression.ty, &field.ty)
            {
                diagnose(
                    &field_initializer.field_name.location,
                    format!(
                        "Cannot coerce field initializer value of type {} to field '{}' type {} in construction of struct {}",
                        field_initializer.expression.ty, field_initializer.field_name.lexeme(), field.ty, type_expression.ty
                    ),
                );
            }
            if let Some(previous) = seen_fields.insert(
                field_initializer.field_name.lexeme(),
                field_initializer.field_name.location.clone(),
            ) {
                let mut diagnostic = InterpreterDiagnostic::new(
                    &field_initializer.field_name.location,
                    format!(
                        "Field {} is already initialized",
                        field_initializer.field_name
                    ),
                );
                diagnostic.add_label(&previous, "Already initialized here".to_string());
                self.diagnostics.push(diagnostic);
            }
        }

        for (name, field) in struct_type.fields.iter().sorted_by_key(|(name, _)| *name) {
            if !seen_fields.contains_key(name) {
                diagnose(
                    ast_info.location,
                    format!(
                        "Struct initializer for struct {} is missing field '{}' of type '{}'",
                        type_expression.ty, name, field.ty,
                    ),
                );
            }
        }
        self.diagnostics.append(&mut diagnostics);
        Ok(())
    }

    fn resolve_assign_expr(
        &mut self,
        assign: &mut AssignExpr<'ws>,
        ast_info: &mut CommonAstInfo<'_, 'ws>,
    ) -> FelicoResult<()> {
        let destination = &assign.destination;
        self.resolve_expr(&mut assign.value)?;
        let distance_and_symbol =
            self.get_definition_distance_and_symbol(&destination.data.parts[0]);
        if let Some((distance, symbol)) = distance_and_symbol {
            assign.distance = distance;
            let destination_type = &symbol.ty;
            *ast_info.ty = symbol.ty;

            let expression_type = &assign.value.ty;
            if !self
                .type_checker
                .is_assignable_to(expression_type, destination_type)
            {
                let mut diagnostic = InterpreterDiagnostic::new(ast_info.location, format!("Expression value of type {} cannot be assigned to variable '{}' of type {}", expression_type, assign.destination, destination_type));
                diagnostic.add_label(
                    &symbol.declaration_site,
                    format!("is declared as {} here", destination_type),
                );
                self.diagnose(diagnostic);
                *ast_info.ty = self.type_factory.unresolved();
            }
        } else {
            self.diagnose(InterpreterDiagnostic::new(
                &destination.location,
                format!("Variable '{}' is not defined here", destination),
            ));
            *ast_info.ty = self.type_factory.unresolved();
        }
        Ok(())
    }

    fn resolve_var_use_expr(
        &mut self,
        var_use: &mut VarUse<'ws>,
        ast_info: &mut CommonAstInfo<'_, 'ws>,
    ) -> FelicoResult<()> {
        let parts = &var_use.name.data.parts;
        let distance_and_symbol = self.get_definition_distance_and_symbol(&parts[0]);
        if let Some((distance, symbol)) = distance_and_symbol {
            var_use.distance = distance;
            let mut ty = symbol.ty;
            for part in parts.iter().skip(1) {
                let methods = symbol.symbol_map.lock().unwrap();
                let Some(sym) = methods.get(part.lexeme()) else {
                    bail!("No symbol found for name {}", var_use.name);
                };
                ty = sym.ty;
            }
            *ast_info.ty = ty;
        } else {
            self.diagnose(InterpreterDiagnostic::new(
                &var_use.name.location,
                format!("Variable '{}' is not defined here", var_use.name),
            ));
            *ast_info.ty = self.type_factory.unresolved();
        }
        Ok(())
    }

    fn resolve_literal_expr(
        &mut self,
        literal: &mut LiteralExpr,
        ast_info: &mut CommonAstInfo<'_, 'ws>,
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
        binary: &mut BinaryExpr<'ws>,
        ast_info: &mut CommonAstInfo<'_, 'ws>,
    ) -> FelicoResult<()> {
        self.resolve_expr(&mut binary.left)?;
        self.resolve_expr(&mut binary.right)?;
        if binary.operator.is_comparison_operator() {
            *ast_info.ty = self.type_factory.bool();
        } else if binary.left.ty == binary.right.ty {
            *ast_info.ty = binary.left.ty;
        }
        Ok(())
    }

    fn resolve_unary_expr(
        &mut self,
        unary: &mut UnaryExpr<'ws>,
        ast_info: &mut CommonAstInfo<'_, 'ws>,
    ) -> FelicoResult<()> {
        self.resolve_expr(&mut unary.right)?;
        *ast_info.ty = unary.right.ty;
        Ok(())
    }

    fn get_definition_distance_and_symbol(
        &self,
        token: &Token<'ws>,
    ) -> Option<(i32, &Symbol<'ws>)> {
        let name = token.lexeme();
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

    fn current_scope(&self) -> &LexicalScope<'ws> {
        self.scopes
            .iter()
            .last()
            .expect("Scope Stack should not be empty")
    }

    fn current_scope_mut(&mut self) -> &mut LexicalScope<'ws> {
        self.scopes
            .iter_mut()
            .last()
            .expect("Scope Stack should not be empty")
    }

    fn add_symbol_to_scope(
        &mut self,
        name: WorkspaceString<'ws>,
        symbol: Symbol<'ws>,
    ) -> FelicoResult<()> {
        match self.current_scope_mut().entry(name) {
            Entry::Occupied(value) => {
                let mut diagnostic = InterpreterDiagnostic::new(
                    &symbol.declaration_site,
                    format!("The name '{}' already declared", name),
                );
                diagnostic.add_label(&value.get().declaration_site, "is already declared here");
                self.diagnose(diagnostic);
            }
            Entry::Vacant(slot) => {
                slot.insert(symbol);
            }
        }
        Ok(())
    }

    fn resolve_return_expr(
        &mut self,
        return_expr: &mut ReturnExpr<'ws>,
        ast: &mut CommonAstInfo<'_, 'ws>,
    ) -> FelicoResult<()> {
        self.resolve_expr(&mut return_expr.expression)?;
        if let Some(Some(current_function_info)) = self
            .scopes
            .iter()
            .rev()
            .map(|symbol| &symbol.current_function)
            .find(|ret| ret.is_some())
        {
            let returned_type = &return_expr.expression.ty;
            let expected_type = &current_function_info.declared_return_type;
            if !self
                .type_checker
                .is_assignable_to(returned_type, expected_type)
            {
                let mut diagnostic = InterpreterDiagnostic::new(
                    &return_expr.expression.location,
                    format!(
                        "Cannot return value of type {} in function returning {}",
                        returned_type, expected_type
                    ),
                );
                diagnostic.add_label(
                    &current_function_info.return_type_declaration_site,
                    format!(
                        "declared with return type {} here",
                        current_function_info.declared_return_type
                    ),
                );
                self.diagnose(diagnostic);
            }
        } else {
            bail!("Cannot return in a non function context");
        }
        *ast.ty = self.type_factory.never();
        Ok(())
    }
}

pub fn resolve_variables<'ws>(
    ast: &mut AstNode<'ws, Module<'ws>>,
    workspace: Workspace<'ws>,
) -> FelicoResult<()> {
    Resolver::new(workspace).resolve_program(ast)
}

#[cfg(test)]
mod tests {
    use crate::frontend::ast::print_ast::AstPrinter;
    use crate::frontend::parse::parser::Parser;
    use crate::frontend::resolve::resolver::{resolve_variables, Resolver};
    use crate::infra::arena::Arena;
    use crate::infra::result::unwrap_error_result_to_string;
    use crate::model::workspace::Workspace;
    use expect_test::{expect, Expect};

    fn test_resolve_program(
        name: &str,
        input: &str,
        expected_ast: Expect,
        expected_manifest: Expect,
    ) {
        let arena = Arena::new();
        let workspace = Workspace::new(&arena);
        let mut parser =
            Parser::new(workspace.source_file_from_string(name, input), workspace).unwrap();
        let mut program = parser.parse_script().unwrap();
        let mut resolver = Resolver::new(workspace);
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
            Module
            └── Declare fun 'main()': ❬Fn() -> ❬Unit❭❭
                ├── Return type: Read 'unit'
                ├── Let ''a' (Identifier)': ❬bool❭
                │   └── Bool(true): ❬bool❭
                └── Unit: ❬Unit❭
        "#]],expect![[r#"
            Module
              main: ❬Fn() -> ❬Unit❭❭
        "#]];

        let_inferred_type: "let a = 3;" => expect![[r#"
            Module
            └── Declare fun 'main()': ❬Fn() -> ❬Unit❭❭
                ├── Return type: Read 'unit'
                ├── Let ''a' (Identifier)': ❬f64❭
                │   └── F64(3.0): ❬f64❭
                └── Unit: ❬Unit❭
        "#]],expect![[r#"
                Module
                  main: ❬Fn() -> ❬Unit❭❭
            "#]];
        let_inferred_type_from_binary_expression: "let a = 1 + 2;" => expect![[r#"
            Module
            └── Declare fun 'main()': ❬Fn() -> ❬Unit❭❭
                ├── Return type: Read 'unit'
                ├── Let ''a' (Identifier)': ❬f64❭
                │   └── +: ❬f64❭
                │       ├── F64(1.0): ❬f64❭
                │       └── F64(2.0): ❬f64❭
                └── Unit: ❬Unit❭
        "#]],expect![[r#"
                Module
                  main: ❬Fn() -> ❬Unit❭❭
            "#]];
        let_inferred_type_from_unary_expression: "let a = -1;" => expect![[r#"
            Module
            └── Declare fun 'main()': ❬Fn() -> ❬Unit❭❭
                ├── Return type: Read 'unit'
                ├── Let ''a' (Identifier)': ❬f64❭
                │   └── -: ❬f64❭
                │       └── F64(1.0): ❬f64❭
                └── Unit: ❬Unit❭
        "#]],expect![[r#"
                Module
                  main: ❬Fn() -> ❬Unit❭❭
            "#]];
        let_inferred_type_from_variable: "let a = 1;let b = a;" => expect![[r#"
            Module
            └── Declare fun 'main()': ❬Fn() -> ❬Unit❭❭
                ├── Return type: Read 'unit'
                ├── Let ''a' (Identifier)': ❬f64❭
                │   └── F64(1.0): ❬f64❭
                ├── Let ''b' (Identifier)': ❬f64❭
                │   └── Read 'a': ❬f64❭
                └── Unit: ❬Unit❭
        "#]],expect![[r#"
            Module
              main: ❬Fn() -> ❬Unit❭❭
        "#]];
        assign_type: "let a = 1;a = 3;" => expect![[r#"
            Module
            └── Declare fun 'main()': ❬Fn() -> ❬Unit❭❭
                ├── Return type: Read 'unit'
                ├── Let ''a' (Identifier)': ❬f64❭
                │   └── F64(1.0): ❬f64❭
                ├── a = : ❬f64❭
                │   └── F64(3.0): ❬f64❭
                └── Unit: ❬Unit❭
        "#]],expect![[r#"
            Module
              main: ❬Fn() -> ❬Unit❭❭
        "#]];
        call_type_native: "sqrt(3);" => expect![[r#"
            Module
            └── Declare fun 'main()': ❬Fn() -> ❬Unit❭❭
                ├── Return type: Read 'unit'
                ├── Call: ❬f64❭
                │   ├── Read 'sqrt': ❬Fn(❬f64❭) -> ❬f64❭❭
                │   └── F64(3.0): ❬f64❭
                └── Unit: ❬Unit❭
        "#]],expect![[r#"
            Module
              main: ❬Fn() -> ❬Unit❭❭
        "#]];
        function_simple: "fun x(a: bool, b: i64) -> f64 {} let a = x;" => expect![[r#"
            Module
            └── Declare fun 'main()': ❬Fn() -> ❬Unit❭❭
                ├── Return type: Read 'unit'
                ├── Declare fun 'x(a, b)': ❬Fn(❬bool❭, ❬i64❭) -> ❬f64❭❭
                │   ├── Param a
                │   │   └── Read 'bool'
                │   ├── Param b
                │   │   └── Read 'i64'
                │   ├── Return type: Read 'f64'
                │   └── Unit: ❬Unit❭
                ├── Let ''a' (Identifier)': ❬Fn(❬bool❭, ❬i64❭) -> ❬f64❭❭
                │   └── Read 'x': ❬Fn(❬bool❭, ❬i64❭) -> ❬f64❭❭
                └── Unit: ❬Unit❭
        "#]],expect![[r#"
            Module
              main: ❬Fn() -> ❬Unit❭❭
        "#]];
        function_with_return: "fun x(a: f64) -> f64 {return a;}" => expect![[r#"
            Module
            └── Declare fun 'main()': ❬Fn() -> ❬Unit❭❭
                ├── Return type: Read 'unit'
                ├── Declare fun 'x(a)': ❬Fn(❬f64❭) -> ❬f64❭❭
                │   ├── Param a
                │   │   └── Read 'f64'
                │   ├── Return type: Read 'f64'
                │   ├── Return: ❬never❭
                │   │   └── Read 'a': ❬f64❭
                │   └── Unit: ❬Unit❭
                └── Unit: ❬Unit❭
        "#]],expect![[r#"
            Module
              main: ❬Fn() -> ❬Unit❭❭
        "#]];
        function_arg_type: "fun x(a: bool, b: i64) -> f64 {
                a;
                b;
            }" => expect![[r#"
                Module
                └── Declare fun 'main()': ❬Fn() -> ❬Unit❭❭
                    ├── Return type: Read 'unit'
                    ├── Declare fun 'x(a, b)': ❬Fn(❬bool❭, ❬i64❭) -> ❬f64❭❭
                    │   ├── Param a
                    │   │   └── Read 'bool'
                    │   ├── Param b
                    │   │   └── Read 'i64'
                    │   ├── Return type: Read 'f64'
                    │   ├── Read 'a': ❬bool❭
                    │   ├── Read 'b': ❬i64❭
                    │   └── Unit: ❬Unit❭
                    └── Unit: ❬Unit❭
            "#]],expect![[r#"
                Module
                  main: ❬Fn() -> ❬Unit❭❭
            "#]];
       program_struct_simple: "
           struct Foo {
                bar: bool,
                baz: f64
            }
           " => expect![[r#"
               Module
               └── Declare fun 'main()': ❬Fn() -> ❬Unit❭❭
                   ├── Return type: Read 'unit'
                   ├── Struct 'Foo': ❬Type❭
                   │   ├── Field bar: ❬bool❭
                   │   │   └── Read 'bool': ❬Type❭
                   │   └── Field baz: ❬f64❭
                   │       └── Read 'f64': ❬Type❭
                   └── Unit: ❬Unit❭
           "#]],expect![[r#"
               Module
                 main: ❬Fn() -> ❬Unit❭❭
           "#]];
        program_struct_empty: "
           struct Empty {}
           " => expect![[r#"
               Module
               └── Declare fun 'main()': ❬Fn() -> ❬Unit❭❭
                   ├── Return type: Read 'unit'
                   ├── Struct 'Empty': ❬Type❭
                   └── Unit: ❬Unit❭
           "#]],expect![[r#"
               Module
                 main: ❬Fn() -> ❬Unit❭❭
           "#]];
        program_struct_impl: "
              struct Something {
                   val: f64,
              }
              impl Something {
                  fun new(val: f64) -> Something {
                    return Something {val: val};
                  }      
              };
            let a = Something::new(123);
           " => expect![[r#"
               Module
               └── Declare fun 'main()': ❬Fn() -> ❬Unit❭❭
                   ├── Return type: Read 'unit'
                   ├── Struct 'Something': ❬Type❭
                   │   └── Field val: ❬f64❭
                   │       └── Read 'f64': ❬Type❭
                   ├── Impl 'Something': ❬Unit❭
                   │   └── Method: Declare fun 'new(val)'
                   │       ├── Param val
                   │       │   └── Read 'f64'
                   │       ├── Return type: Read 'Something'
                   │       ├── Return: ❬never❭
                   │       │   └── Create struct: ❬Something❭
                   │       │       ├── Read 'Something': ❬Something❭
                   │       │       └── val: Read 'val': ❬f64❭
                   │       └── Unit: ❬Unit❭
                   ├── Let ''a' (Identifier)': ❬Something❭
                   │   └── Call: ❬Something❭
                   │       ├── Read 'Something::new': ❬Fn(❬f64❭) -> ❬Something❭❭
                   │       └── F64(123.0): ❬f64❭
                   └── Unit: ❬Unit❭
           "#]],expect![[r#"
               Module
                 main: ❬Fn() -> ❬Unit❭❭
           "#]];
    );
    fn test_resolve_program_error(name: &str, input: &str, expected: Expect) {
        let arena = Arena::new();
        let workspace = Workspace::new(&arena);
        let mut parser =
            Parser::new(workspace.source_file_from_string(name, input), workspace).unwrap();
        let mut ast = parser.parse_script().unwrap();
        let result = resolve_variables(&mut ast, workspace);
        let diagnostic_string = unwrap_error_result_to_string(&result);
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
               ╰────

        "#]];
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
               ╰────

        "#]];
        double_declaration: "let x = 0;\ndebug_print(x);\nlet x = true;" => expect![[r#"
            × The name 'x' already declared
               ╭─[double_declaration:3:5]
             1 │ let x = 0;
               ·     ┬
               ·     ╰── is already declared here
             2 │ debug_print(x);
             3 │ let x = true;
               ·     ─
               ╰────

        "#]];
        double_declaration_with_function: "let x = 0;\nfun x() {}" => expect![[r#"
            × The name 'x' already declared
               ╭─[double_declaration_with_function:2:5]
             1 │ let x = 0;
               ·     ┬
               ·     ╰── is already declared here
             2 │ fun x() {}
               ·     ─
               ╰────

        "#]];
        use_undefined_variable: "debug_print(x);" => expect![[r#"
            × Variable 'x' is not defined here
               ╭─[use_undefined_variable:1:13]
             1 │ debug_print(x);
               ·             ─
               ╰────

        "#]];
        assign_undefined_variable: "x = 3;" => expect![[r#"
            × Variable 'x' is not defined here
               ╭─[assign_undefined_variable:1:1]
             1 │ x = 3;
               · ─
               ╰────

        "#]];
        call_undefined_function: "x();" => expect![[r#"
            × Variable 'x' is not defined here
               ╭─[call_undefined_function:1:1]
             1 │ x();
               · ─
               ╰────

            × Expected a function to call, but instead found type ❬unresolved❭
               ╭─[call_undefined_function:1:1]
             1 │ x();
               · ─
               ╰────

        "#]];
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
                ╰────

         "#]];
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
                ╰────

         "#]];
        call_a_non_function: r#"
        true();
        "# => expect![[r#"
            × Expected a function to call, but instead found type ❬bool❭
               ╭─[call_a_non_function:2:9]
             1 │ 
             2 │         true();
               ·         ────
             3 │         
               ╰────

        "#]];
        call_wrong_number_of_arguments: r#"
        sqrt(1,2);
        "# => expect![[r#"
            × Wrong number of arguments in call - expected: 1, actual 2
               ╭─[call_wrong_number_of_arguments:2:9]
             1 │ 
             2 │         sqrt(1,2);
               ·         ──────────
             3 │         
               ╰────

        "#]];

        call_with_wrong_argument: r#"
        fun foo() {};
        foo(true);
        "# => expect![[r#"
            × Wrong number of arguments in call - expected: 0, actual 1
               ╭─[call_with_wrong_argument:3:9]
             1 │ 
             2 │         fun foo() {};
               ·             ─┬─
               ·              ╰── Function declared here
             3 │         foo(true);
               ·         ──────────
             4 │         
               ╰────

        "#]];
        return_wrong_type: r#"
        fun foo() -> bool {return 3;}
        "# => expect![[r#"
            × Cannot return value of type ❬f64❭ in function returning ❬bool❭
               ╭─[return_wrong_type:2:35]
             1 │ 
             2 │         fun foo() -> bool {return 3;}
               ·                      ──┬─         ─
               ·                        ╰── declared with return type ❬bool❭ here
             3 │         
               ╰────

        "#]];
        multiple_diagnostics: "a+b;" => expect![[r#"
            × Variable 'a' is not defined here
               ╭─[multiple_diagnostics:1:1]
             1 │ a+b;
               · ─
               ╰────

            × Variable 'b' is not defined here
               ╭─[multiple_diagnostics:1:3]
             1 │ a+b;
               ·   ─
               ╰────

        "#]];
        multiple_diagnostics_followup_error: "let x = a+3;x+3;" => expect![[r#"
            × Variable 'a' is not defined here
               ╭─[multiple_diagnostics_followup_error:1:9]
             1 │ let x = a+3;x+3;
               ·         ─
               ╰────

        "#]];
        different_if_types: "if (true) 3 else true;" => expect![[r#"
            × Then and else branch of if statement must evaluate to the same type, but then evaluates to ❬f64❭, while else evaluates to ❬bool❭
               ╭─[different_if_types:1:1]
             1 │ if (true) 3 else true;
               · ──────────────────────
               ╰────

        "#]];
        different_if_types_no_else: "if (true) 3;" => expect![[r#"
            × Then and else branch of if statement must evaluate to the same type, but then evaluates to ❬f64❭, while else evaluates to ❬Unit❭
               ╭─[different_if_types_no_else:1:1]
             1 │ if (true) 3;
               · ────────────
               ╰────

        "#]];
        wrong_type_in_if_condition: "if (3) {};" => expect![[r#"
            × Condition in if statement must evaluate to a boolean, but was of type ❬f64❭ instead
               ╭─[wrong_type_in_if_condition:1:5]
             1 │ if (3) {};
               ·     ─
               ╰────

        "#]];

        wrong_struct_type: "struct Foo {}; if (Foo {}) {};" => expect![[r#"
            × Condition in if statement must evaluate to a boolean, but was of type ❬Foo❭ instead
               ╭─[wrong_struct_type:1:20]
             1 │ struct Foo {}; if (Foo {}) {};
               ·                    ───────
               ╰────

        "#]];

        struct_initialize_to_non_existant_fields: "struct Foo {}; Foo {a: 3, b: true};" => expect![[r#"
            × Struct ❬Foo❭ does not have a field named 'a'
               ╭─[struct_initialize_to_non_existant_fields:1:21]
             1 │ struct Foo {}; Foo {a: 3, b: true};
               ·        ─┬─          ─
               ·         ╰── Struct declared here
               ╰────

            × Struct ❬Foo❭ does not have a field named 'b'
               ╭─[struct_initialize_to_non_existant_fields:1:27]
             1 │ struct Foo {}; Foo {a: 3, b: true};
               ·        ─┬─                ─
               ·         ╰── Struct declared here
               ╰────

        "#]];

        struct_initialize_with_wrong_types: "struct Foo {a: bool, b: i64,}; Foo {a: 3, b: true};" => expect![[r#"
            × Cannot coerce field initializer value of type ❬f64❭ to field 'a' type ❬bool❭ in construction of struct ❬Foo❭
               ╭─[struct_initialize_with_wrong_types:1:37]
             1 │ struct Foo {a: bool, b: i64,}; Foo {a: 3, b: true};
               ·        ─┬─                          ─
               ·         ╰── Struct declared here
               ╰────

            × Cannot coerce field initializer value of type ❬bool❭ to field 'b' type ❬i64❭ in construction of struct ❬Foo❭
               ╭─[struct_initialize_with_wrong_types:1:43]
             1 │ struct Foo {a: bool, b: i64,}; Foo {a: 3, b: true};
               ·        ─┬─                                ─
               ·         ╰── Struct declared here
               ╰────

        "#]];      
        struct_initialize_missing_fields: "struct Foo {a: bool, b: i64,}; Foo {};" => expect![[r#"
            × Struct initializer for struct ❬Foo❭ is missing field 'a' of type '❬bool❭'
               ╭─[struct_initialize_missing_fields:1:32]
             1 │ struct Foo {a: bool, b: i64,}; Foo {};
               ·        ─┬─                     ───────
               ·         ╰── Struct declared here
               ╰────

            × Struct initializer for struct ❬Foo❭ is missing field 'b' of type '❬i64❭'
               ╭─[struct_initialize_missing_fields:1:32]
             1 │ struct Foo {a: bool, b: i64,}; Foo {};
               ·        ─┬─                     ───────
               ·         ╰── Struct declared here
               ╰────

        "#]];

        struct_initialize_field_twice: "struct Foo {a: bool}; Foo {a: true, a: false};" => expect![[r#"
            × Field 'a' (Identifier) is already initialized
               ╭─[struct_initialize_field_twice:1:37]
             1 │ struct Foo {a: bool}; Foo {a: true, a: false};
               ·                            ┬        ─
               ·                            ╰── Already initialized here
               ╰────

        "#]];

        struct_get_on_wrong_type: "debug_print.foo;" => expect![[r#"
            × Type ❬Fn(❬any❭) -> ❬Unit❭❭ has no field 'foo'
               ╭─[struct_get_on_wrong_type:1:1]
             1 │ debug_print.foo;
               · ────────────────
               ╰────

        "#]];

        struct_get_wrong_name: "struct Foo {a: f64}; Foo {a: 3}.b;" => expect![[r#"
            × Type ❬Foo❭ has no field 'b'
               ╭─[struct_get_wrong_name:1:22]
             1 │ struct Foo {a: f64}; Foo {a: 3}.b;
               ·                      ─────────────
               ╰────

        "#]];

        struct_get_wrong_type: "struct Foo {a: f64}; if(Foo {a: 3}.a) {};" => expect![[r#"
            × Condition in if statement must evaluate to a boolean, but was of type ❬f64❭ instead
               ╭─[struct_get_wrong_type:1:25]
             1 │ struct Foo {a: f64}; if(Foo {a: 3}.a) {};
               ·                         ─────────────
               ╰────

        "#]];
        struct_set_wrong_type: "struct Foo {a: f64}; Foo {a: 3}.a = true;" => expect![[r#"
            × Expression value of type ❬bool❭ cannot be assigned to field '❬Foo❭.a' of type ❬f64❭
               ╭─[struct_set_wrong_type:1:22]
             1 │ struct Foo {a: f64}; Foo {a: 3}.a = true;
               ·                      ────────────────────
               ╰────

        "#]];
    );
}
