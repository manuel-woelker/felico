use crate::frontend::ast::expr::{
    AssignExpr, BinaryExpr, BlockExpr, CallExpr, Expr, GetExpr, IfExpr, LiteralExpr, ReturnExpr,
    SetExpr, UnaryExpr, VarUse,
};
use crate::frontend::ast::node::AstNode;
use crate::frontend::ast::program::Program;
use crate::frontend::ast::stmt::{FunStmt, Stmt, WhileStmt};
use crate::frontend::ast::AstData;
use crate::frontend::lex::token::TokenType;
use crate::frontend::parse::parser::{parse_expression, parse_program};
use crate::frontend::resolve::resolver::resolve_variables;
use crate::infra::diagnostic::InterpreterDiagnostic;
use crate::infra::result::FelicoResult;
use crate::infra::source_file::SourceFileHandle;
use crate::interpret::core_definitions::{get_core_definitions, TypeFactory};
use crate::interpret::environment::Environment;
use crate::interpret::value::{
    Callable, CallableFun, DefinedFunction, InterpreterValue, ValueFactory, ValueKind,
};
use std::ops::Deref;
use std::rc::Rc;

type PrintFn = Box<dyn Fn(&InterpreterValue)>;

pub struct Interpreter {
    source_file: SourceFileHandle,
    type_factory: TypeFactory,
    value_factory: ValueFactory,
    environment: Environment,
    print_fn: PrintFn,
    fuel: i64,
    available_stack: i64,
}

macro_rules! check_early_return {
    ($expr:expr) => {
        if $expr.is_return() {
            return Ok($expr);
        }
    };
}

impl InterpreterValue {
    fn is_return(&self) -> bool {
        matches!(self.val, ValueKind::Return(_))
    }
    fn val(value: InterpreterValue) -> Self {
        value
    }
}

impl Interpreter {
    pub fn new(source_file: SourceFileHandle) -> FelicoResult<Self> {
        let mut environment = Environment::new();
        let type_factory = TypeFactory::new();
        for core_definition in get_core_definitions(&type_factory) {
            environment.define(&core_definition.name, core_definition.value.clone());
        }
        environment.enter_new();
        Ok(Self {
            source_file,
            environment,
            print_fn: Box::new(|value| {
                print!("{}", value);
            }),
            fuel: 1000,
            available_stack: 20,
            value_factory: ValueFactory::new(&type_factory),
            type_factory,
        })
    }

    pub fn set_print_fn(&mut self, print_fn: PrintFn) {
        self.print_fn = print_fn;
    }

    pub fn print(&self, value: &InterpreterValue) {
        (self.print_fn)(value);
    }

    pub fn set_fuel(&mut self, fuel: i64) {
        self.fuel = fuel;
    }

    fn cont(&self) -> InterpreterValue {
        self.value_factory.unit()
    }

    fn ret(&self, value: InterpreterValue) -> InterpreterValue {
        InterpreterValue {
            val: ValueKind::Return(Box::new(value)),
            ty: self.type_factory.never(),
        }
    }

    fn spend_fuel<T: AstData>(&mut self, stmt: &AstNode<T>) -> FelicoResult<()> {
        self.fuel -= 1;
        if self.fuel <= 0 {
            return self.create_diagnostic(
                stmt,
                "Out of fuel! Execution took to many loops/function calls.",
                |diagnostic| {
                    diagnostic.add_primary_label(&stmt.location);
                },
            );
        }
        Ok(())
    }

    pub fn evaluate_program(mut self) -> FelicoResult<()> {
        let mut program = parse_program(self.source_file.clone(), &self.type_factory)?;
        resolve_variables(&mut program, &self.type_factory)?;
        self.evaluate_prgrm(&program)
    }

    fn evaluate_prgrm(&mut self, program: &AstNode<Program>) -> FelicoResult<()> {
        self.evaluate_stmts(&program.data.stmts)?;
        Ok(())
    }

    pub fn evaluate_expression(mut self) -> FelicoResult<InterpreterValue> {
        let expr = parse_expression(self.source_file.clone())?;
        self.evaluate_expr(&expr)
    }

    fn evaluate_expr(&mut self, expr: &AstNode<Expr>) -> FelicoResult<InterpreterValue> {
        Ok(InterpreterValue::val(match expr.data.deref() {
            Expr::Literal(literal_expr) => self.evaluate_literal_expr(literal_expr)?,
            Expr::Unary(unary) => self.evaluate_unary_expr(expr, unary)?,
            Expr::Binary(binary) => self.evaluate_binary_expr(expr, binary)?,
            Expr::Variable(var_use) => self.evaluate_var_use_expr(var_use)?,
            Expr::Assign(assign) => self.evaluate_assign_expr(assign)?,
            Expr::Return(return_stmt) => self.evaluate_return_expr(return_stmt)?,
            Expr::Block(block) => self.evaluate_block_expr(block)?,
            Expr::If(if_expr) => self.evaluate_if_expr(if_expr)?,
            Expr::Get(get_expr) => self.evaluate_get_expr(get_expr)?,
            Expr::Set(set_expr) => self.evaluate_set_expr(set_expr)?,
            Expr::Call(call) => self.evaluate_call_expr(expr, call)?,
        }))
    }

    fn evaluate_call_expr(
        &mut self,
        expr: &AstNode<Expr>,
        call: &CallExpr,
    ) -> FelicoResult<InterpreterValue> {
        self.available_stack -= 1;
        if self.available_stack <= 0 {
            return self.create_diagnostic(expr, "Stack size exceeded.", |diagnostic| {
                diagnostic.add_primary_label(&expr.location);
            });
        }
        let callee = self.evaluate_expr(&call.callee)?;
        if callee.is_return() {
            self.available_stack += 1;
            return Ok(callee);
        }
        if let ValueKind::Callable(callable) = callee.val {
            // Check arity
            if call.arguments.len() != callable.arity {
                return self.create_diagnostic(expr, format!("Wrong number of arguments in function call '{}' - Expected: {}, got: {} instead", callable.name, callable.arity, call.arguments.len()), |diagnostic| {
                    diagnostic.add_primary_label(&call.callee.location);
                    if let CallableFun::Defined(fun) = callable.fun.as_ref() {
                        diagnostic.add_label(&fun.fun_stmt.name.location, format!("'{}' defined here", callable.name));
                    }
                });
            }
            let mut arguments: Vec<InterpreterValue> = vec![];
            for expr in &call.arguments {
                let argument = self.evaluate_expr(expr)?;
                check_early_return!(argument);
                arguments.push(argument);
            }
            let result = match &callable.fun.as_ref() {
                CallableFun::Native(fun) => match fun(self, arguments) {
                    Ok(result) => result,
                    Err(err) => {
                        return self.create_diagnostic(
                            expr,
                            format!("Error in native call to {}(): {}", callable.name, err),
                            |diagnostic| {
                                diagnostic.add_primary_label(&call.callee.location);
                            },
                        );
                    }
                },
                CallableFun::Defined(defined_function) => {
                    let old_environment = self.environment.clone();
                    self.environment = defined_function.closure.clone().child_environment();
                    defined_function
                        .fun_stmt
                        .parameters
                        .iter()
                        .zip(arguments)
                        .for_each(|(parameter, value)| {
                            self.environment.define(parameter.name.lexeme(), value);
                        });
                    let result = self.evaluate_expr(&defined_function.fun_stmt.body)?;
                    // this is a return itself
                    let value = if let ValueKind::Return(value) = result.val {
                        *value
                    } else {
                        result
                    };
                    self.environment = old_environment;
                    self.available_stack += 1;
                    return Ok(value);
                }
            };
            self.available_stack += 1;
            Ok(result)
        } else {
            return self.create_diagnostic(
                expr,
                format!("Expression '{:?}' is not callable", callee),
                |diagnostic| {
                    diagnostic.add_primary_label(&call.callee.location);
                },
            );
        }
    }

    fn evaluate_get_expr(&mut self, _get_expr: &GetExpr) -> FelicoResult<InterpreterValue> {
        todo!("Get not supported");
        /*
        let object = self.evaluate_expr(&get.object)?;
        return if let self.value_factory.Object(instance) = &object {
            if let Some(value) = instance.borrow().fields.get(get.name.lexeme()) {
                return Ok(value.clone());
            }
            if let Some(method) = instance.borrow().class.method_map.get(get.name.lexeme()) {
                if let self.value_factory.Callable(callable) = &method {
                    if let CallableFun::Defined(fun) = &*callable.fun {
                        let closure = fun.closure.child_environment();
                        closure.define("this", object.clone());
                        return Ok(self.value_factory.Callable(Callable {
                            name: callable.name.clone(),
                            arity: callable.arity,
                            fun: Rc::new(CallableFun::Defined(DefinedFunction {
                                fun_stmt: fun.fun_stmt.clone(),
                                closure,
                            })),
                        }))
                    } else {
                        bail!("Defined function expected");
                    }
                } else {
                    bail!("Callable expected");
                }
            }
            self.create_diagnostic(expr, format!("No property '{}' found on object {:?}", get.name.lexeme(), instance), |diagnostic| {
                diagnostic.add_primary_label(&expr.location);
            })
        } else {
            self.create_diagnostic(expr, format!("Expected object for dot access instead found {:?}", &object), |diagnostic| {
                diagnostic.add_primary_label(&expr.location);
            })
        }
         */
    }

    fn evaluate_set_expr(&mut self, _set_expr: &SetExpr) -> FelicoResult<InterpreterValue> {
        todo!("Get not supported");
        /*
        let object = self.evaluate_expr(&set.object)?;
        return if let self.value_factory.Object(instance) = object {
            let value = self.evaluate_expr(&set.value)?;
            instance.borrow_mut().fields.insert(set.name.lexeme().to_string(), value.clone());
            Ok(value)
        } else {
            self.create_diagnostic(expr, format!("Expected object for dot access instead found {:?}", &object), |diagnostic| {
                diagnostic.add_primary_label(&expr.location);
            })
        }*/
    }

    fn evaluate_if_expr(&mut self, if_expr: &IfExpr) -> FelicoResult<InterpreterValue> {
        let condition = self.evaluate_expr(&if_expr.condition)?;
        check_early_return!(condition);
        match condition.val {
            ValueKind::Bool(true) => {
                let then_result = self.evaluate_expr(&if_expr.then_expr)?;
                Ok(then_result)
            }
            ValueKind::Bool(false) => {
                if let Some(else_expr) = &if_expr.else_expr {
                    self.evaluate_expr(else_expr)
                } else {
                    Ok(InterpreterValue::val(self.value_factory.unit()))
                }
            }
            other => self.create_diagnostic(
                &if_expr.condition,
                format!(
                    "Expected true or false in if condition, but found '{}' instead",
                    other
                ),
                |diagnostic| {
                    diagnostic.add_primary_label(&if_expr.condition.location);
                },
            ),
        }
    }

    fn evaluate_block_expr(&mut self, block: &BlockExpr) -> FelicoResult<InterpreterValue> {
        self.environment.enter_new();
        let stmt_result = self.evaluate_stmts(&block.stmts[..])?;
        if stmt_result.is_return() {
            self.environment.exit();
            return Ok(stmt_result);
        }
        let result = self.evaluate_expr(&block.result_expression)?;
        self.environment.exit();
        Ok(result)
    }

    fn evaluate_return_expr(&mut self, return_stmt: &ReturnExpr) -> FelicoResult<InterpreterValue> {
        let return_result = self.evaluate_expr(&return_stmt.expression)?;
        check_early_return!(return_result);
        Ok(self.ret(return_result))
    }

    fn evaluate_assign_expr(&mut self, assign: &AssignExpr) -> FelicoResult<InterpreterValue> {
        let value = self.evaluate_expr(&assign.value)?;
        check_early_return!(value);
        self.environment.assign_at_distance(
            assign.destination.lexeme(),
            assign.distance,
            value.clone(),
        )?;
        Ok(value)
    }

    fn evaluate_var_use_expr(&mut self, var_use: &VarUse) -> FelicoResult<InterpreterValue> {
        Ok(self
            .environment
            .get_at_distance(var_use.variable.lexeme(), var_use.distance)?
            .clone())
    }

    fn evaluate_binary_expr(
        &mut self,
        expr: &AstNode<Expr>,
        binary: &BinaryExpr,
    ) -> FelicoResult<InterpreterValue> {
        let left_value = self.evaluate_expr(&binary.left)?;
        check_early_return!(left_value);
        // Handle "and" & "or" upfront to handle short-circuiting logic
        match binary.operator.token_type {
            TokenType::Or | TokenType::And => {
                if let ValueKind::Bool(left) = left_value.val {
                    if binary.operator.token_type == TokenType::Or {
                        if left {
                            return Ok(InterpreterValue::val(self.value_factory.bool(true)));
                        }
                    } else {
                        // AND
                        if !left {
                            return Ok(InterpreterValue::val(self.value_factory.bool(false)));
                        }
                    }
                    let right_value = self.evaluate_expr(&binary.right)?;
                    check_early_return!(right_value);
                    return match right_value.val {
                        ValueKind::Bool(_) => Ok(InterpreterValue::val(right_value)), // Ok
                        _ => self.create_diagnostic(
                            expr,
                            format!(
                                "Unsupported operand for boolean {} operation: {}",
                                binary.operator.token_type, right_value
                            ),
                            |diagnostic| {
                                diagnostic.add_primary_label(&binary.right.location);
                            },
                        ),
                    };
                } else {
                    return self.create_diagnostic(
                        expr,
                        format!(
                            "Unsupported operand for boolean {} operation: {}",
                            binary.operator.token_type, left_value
                        ),
                        |diagnostic| {
                            diagnostic.add_primary_label(&binary.left.location);
                        },
                    );
                }
            }
            _ => {}
        };
        let right_value = self.evaluate_expr(&binary.right)?;
        check_early_return!(right_value);
        Ok(match (left_value.val, right_value.val) {
            (ValueKind::F64(left), ValueKind::F64(right)) => match binary.operator.token_type {
                TokenType::Minus => self.value_factory.f64(left - right),
                TokenType::Plus => self.value_factory.f64(left + right),
                TokenType::Star => self.value_factory.f64(left * right),
                TokenType::Slash => self.value_factory.f64(left / right),
                TokenType::EqualEqual => self.value_factory.bool(left == right),
                TokenType::BangEqual => self.value_factory.bool(left != right),
                TokenType::Greater => self.value_factory.bool(left > right),
                TokenType::GreaterEqual => self.value_factory.bool(left >= right),
                TokenType::Less => self.value_factory.bool(left < right),
                TokenType::LessEqual => self.value_factory.bool(left <= right),
                _ => {
                    return self.create_diagnostic(
                        expr,
                        format!(
                            "Unsupported binary operator for numbers: {}",
                            binary.operator.lexeme()
                        ),
                        |diagnostic| {
                            diagnostic.add_primary_label(&binary.operator.location);
                        },
                    );
                }
            },
            (ValueKind::String(left), right) => match binary.operator.token_type {
                TokenType::Plus => {
                    return Ok(InterpreterValue::val(
                        self.value_factory.new_string(left + &format!("{}", right)),
                    ))
                }
                _ => {
                    return self.create_diagnostic(
                        expr,
                        format!(
                            "Unsupported binary operator for string: {}",
                            binary.operator.lexeme()
                        ),
                        |diagnostic| {
                            diagnostic.add_primary_label(&binary.operator.location);
                        },
                    );
                }
            },
            (left, right) => {
                return self.create_diagnostic(
                    expr,
                    format!(
                        "Operator {:?} not defined for values {:?} and {:?}",
                        binary.operator.token_type, left, right
                    ),
                    |diagnostic| {
                        diagnostic.add_primary_label(&expr.location);
                    },
                );
            }
        })
    }

    fn evaluate_unary_expr(
        &mut self,
        expr: &AstNode<Expr>,
        unary: &UnaryExpr,
    ) -> FelicoResult<InterpreterValue> {
        let sub_expression = self.evaluate_expr(&unary.right)?;
        if sub_expression.is_return() {
            return Ok(sub_expression);
        }
        Ok(match unary.operator.token_type {
            TokenType::Minus => match sub_expression.val {
                ValueKind::F64(number) => self.value_factory.f64(-number),
                _ => {
                    return self.create_diagnostic(
                        expr,
                        format!("Value '{:?}' cannot be negated", sub_expression),
                        |diagnostic| {
                            diagnostic.add_primary_label(&expr.location);
                        },
                    );
                }
            },
            _ => {
                return self.create_diagnostic(
                    expr,
                    format!("Unsupported unary operator {}", unary.operator.token_type),
                    |diagnostic| {
                        diagnostic.add_primary_label(&expr.location);
                    },
                );
            }
        })
    }

    fn create_diagnostic<T, S: Into<String>, A: AstData>(
        &self,
        ast_node: &AstNode<A>,
        message: S,
        mut f: impl FnMut(&mut InterpreterDiagnostic),
    ) -> FelicoResult<T> {
        let mut diagnostic =
            InterpreterDiagnostic::new(&ast_node.location.source_file, message.into());
        f(&mut diagnostic);
        Err(diagnostic.into())
    }

    fn evaluate_stmt(&mut self, stmt: &AstNode<Stmt>) -> FelicoResult<InterpreterValue> {
        match stmt.data.deref() {
            Stmt::Expression(expr) => {
                let expr_result = self.evaluate_expr(&expr.expression)?;
                check_early_return!(expr_result);
            }
            Stmt::Let(var) => {
                let value = self.evaluate_expr(&var.expression)?;
                check_early_return!(value);
                self.environment.define(var.name.lexeme(), value);
            }
            Stmt::Fun(fun) => {
                self.self_evaluate_fun_stmt(stmt, fun);
            }
            Stmt::While(while_stmt) => {
                let result = self.evaluate_while_stmt(&while_stmt)?;
                check_early_return!(result);
            }
            Stmt::Struct(_) => {
                todo!("implement struct");
            }
        }
        Ok(self.cont())
    }

    fn evaluate_while_stmt(&mut self, while_stmt: &&WhileStmt) -> FelicoResult<InterpreterValue> {
        loop {
            let condition = self.evaluate_expr(&while_stmt.condition)?;
            check_early_return!(condition);
            match condition.val {
                ValueKind::Bool(true) => {
                    let result = self.evaluate_stmt(&while_stmt.body_stmt)?;
                    check_early_return!(result);
                }
                ValueKind::Bool(false) => {
                    break;
                }
                other => {
                    return self.create_diagnostic(
                        &while_stmt.condition,
                        format!(
                            "Expected true or false in loop condition, but found '{}' instead",
                            other
                        ),
                        |diagnostic| {
                            diagnostic.add_primary_label(&while_stmt.condition.location);
                        },
                    );
                }
            }
            self.spend_fuel(&while_stmt.condition)?;
        }
        Ok(self.value_factory.unit())
    }

    fn self_evaluate_fun_stmt(&mut self, stmt: &AstNode<Stmt>, fun: &FunStmt) {
        let callable = self.create_fun_callable(fun, stmt);
        self.environment.define(fun.name.lexeme(), callable);
    }

    fn create_fun_callable(&mut self, fun: &FunStmt, stmt: &AstNode<Stmt>) -> InterpreterValue {
        let callable = self.value_factory.callable(
            Callable {
                name: fun.name.lexeme().to_string(),
                arity: fun.parameters.len(),
                fun: Rc::new(CallableFun::Defined(DefinedFunction {
                    fun_stmt: fun.clone(),
                    closure: self.environment.clone(),
                })),
            },
            stmt.ty.clone(),
        );
        callable
    }

    fn evaluate_stmts(&mut self, stmts: &[AstNode<Stmt>]) -> FelicoResult<InterpreterValue> {
        for stmt in stmts {
            let result = self.evaluate_stmt(stmt)?;
            if result.is_return() {
                return Ok(result);
            }
        }
        Ok(InterpreterValue::val(self.value_factory.unit()))
    }

    fn evaluate_literal_expr(&self, literal_expr: &LiteralExpr) -> FelicoResult<InterpreterValue> {
        Ok(match literal_expr {
            LiteralExpr::Unit => self.value_factory.unit(),
            LiteralExpr::Str(string) => self.value_factory.new_string(string.clone()),
            LiteralExpr::F64(number) => self.value_factory.f64(*number),
            LiteralExpr::I64(number) => self.value_factory.i64(*number),
            LiteralExpr::Bool(bool) => self.value_factory.bool(*bool),
        })
    }
}

pub fn run_program_to_string(name: &str, input: &str) -> FelicoResult<String> {
    let mut interpreter = Interpreter::new(SourceFileHandle::from_string(name, input))?;
    let output_buffer = std::sync::Arc::new(std::sync::RwLock::new(String::new()));
    let output_buffer_clone = output_buffer.clone();
    interpreter.set_print_fn(Box::new(move |value| {
        output_buffer
            .write()
            .unwrap()
            .push_str(&format!("{}", value))
    }));
    interpreter.evaluate_program()?;
    let guard = output_buffer_clone.write().unwrap();
    Ok(guard.deref().clone())
}

#[cfg(test)]
mod tests {
    use crate::infra::diagnostic::unwrap_diagnostic_to_string;
    use crate::infra::source_file::SourceFileHandle;
    use crate::interpret::eval::eval_expression;
    use crate::interpret::interpreter::{run_program_to_string, Interpreter};
    use expect_test::{expect, Expect};

    fn test_eval_expression(name: &str, input: &str, expected: Expect) {
        let result = eval_expression(SourceFileHandle::from_string(name, input)).unwrap();
        expected.assert_eq(&format!("{:?}", result));
    }

    macro_rules! test_eval_expression {
    ( $($label:ident: $input:expr => $expect:expr;)+ ) => {
        $(
            #[test]
            fn $label() {
                test_eval_expression(stringify!($label), $input, $expect);
            }
        )*
        }
    }

    test_eval_expression!(
        literal_number_0: "0" => expect!["F64(0.0)"];
    );

    fn test_eval_program(name: &str, input: &str, expected: Expect) {
        let result = run_program_to_string(name, input).unwrap();
        expected.assert_eq(&result);
    }

    macro_rules! test_eval_program {
    ( $($label:ident: $input:expr => $expect:expr;)+ ) => {
        $(
            #[test]
            fn $label() {
                test_eval_program(stringify!($label), $input, $expect);
            }
        )*
        }
    }

    test_eval_program!(
        program_fib: "
            fun fib(n: f64) -> f64 {
                 return if (n <= 1) n else
                 fib(n - 2) + fib(n - 1);
            }
            debug_print(fib(6));
        " => expect!["8"];
    );

    fn test_interpret_program_error(name: &str, input: &str, expected: Expect) {
        let mut interpreter = Interpreter::new(SourceFileHandle::from_string(name, input)).unwrap();
        interpreter.set_print_fn(Box::new(move |_value| {}));
        let result = interpreter.evaluate_program();
        let diagnostic_string = unwrap_diagnostic_to_string(&result);
        expected.assert_eq(&diagnostic_string);
    }

    macro_rules! test_interpret_error {
    ( $($label:ident: $input:expr => $expect:expr;)+ ) => {
        $(
            #[test]
            fn $label() {
                test_interpret_program_error(stringify!($label), $input, $expect);
            }
        )*
        }
    }

    test_interpret_error!(
        invalid_addition: "debug_print(3+true);" => expect![[r#"
            × Operator Plus not defined for values F64(3.0) and Bool(true)
               ╭─[invalid_addition:1:13]
             1 │ debug_print(3+true);
               ·             ───────
               ╰────"#]];
        call_uncallable: "true();" => expect![[r#"
            × Expected a function to call, but instead found type ❬bool❭
               ╭─[call_uncallable:1:1]
             1 │ true();
               · ────
               ╰────"#]];
        call_wrong_arity: "sqrt();" => expect![[r#"
            × Wrong number of arguments in call - expected: 1, actual 0
               ╭─[call_wrong_arity:1:5]
             1 │ sqrt();
               ·     ───
               ╰────"#]];
        call_wrong_arity_defined: "fun foo(a: bool) {}\ndebug_print(3);\nfoo();" => expect![[r#"
            × Wrong number of arguments in call - expected: 1, actual 0
               ╭─[call_wrong_arity_defined:3:4]
             2 │ debug_print(3);
             3 │ foo();
               ·    ───
               ╰────"#]];
        wrong_string_operator: "\"foo\" * 3;" => expect![[r#"
            × Unsupported binary operator for string: *
               ╭─[wrong_string_operator:1:7]
             1 │ "foo" * 3;
               ·       ─
               ╰────"#]];
        wrong_boolean_operand1: "3 || true;" => expect![[r#"
            × Unsupported operand for boolean Or operation: 3
               ╭─[wrong_boolean_operand1:1:1]
             1 │ 3 || true;
               · ─
               ╰────"#]];
        wrong_boolean_operand2: "false || 3;" => expect![[r#"
            × Unsupported operand for boolean Or operation: 3
               ╭─[wrong_boolean_operand2:1:10]
             1 │ false || 3;
               ·          ─
               ╰────"#]];
        wrong_negation_operand: "-true;" => expect![[r#"
            × Value 'Bool(true)' cannot be negated
               ╭─[wrong_negation_operand:1:1]
             1 │ -true;
               · ──────
               ╰────"#]];
        wrong_bang_operand: "!3;" => expect![[r#"
            × Unsupported unary operator Bang
               ╭─[wrong_bang_operand:1:1]
             1 │ !3;
               · ───
               ╰────"#]];
        wrong_type_in_if: "if(3) {}" => expect![[r#"
            × Expected true or false in if condition, but found '3' instead
               ╭─[wrong_type_in_if:1:4]
             1 │ if(3) {}
               ·    ─
               ╰────"#]];
        wrong_type_in_while: "while(3) {}" => expect![[r#"
            × Expected true or false in loop condition, but found '3' instead
               ╭─[wrong_type_in_while:1:7]
             1 │ while(3) {}
               ·       ─
               ╰────"#]];
        sqrt_true: "sqrt(true);" => expect![[r#"
            × Cannot coerce argument of type ❬bool❭ as parameter of type ❬f64❭ in function invocation
               ╭─[sqrt_true:1:6]
             1 │ sqrt(true);
               ·      ────
               ╰────"#]];
        endless_loop: "while(true) {}" => expect![[r#"
            × Out of fuel! Execution took to many loops/function calls.
               ╭─[endless_loop:1:7]
             1 │ while(true) {}
               ·       ────
               ╰────"#]];
        endless_recursion: "fun a() {a();} a();" => expect![[r#"
            × Stack size exceeded.
               ╭─[endless_recursion:1:11]
             1 │ fun a() {a();} a();
               ·           ───
               ╰────"#]];
    );
}
