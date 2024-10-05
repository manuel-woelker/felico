use crate::frontend::ast::AstData;
use crate::frontend::ast::expr::{Expr, LiteralExpr};
use crate::frontend::ast::node::AstNode;
use crate::frontend::ast::program::Program;
use crate::frontend::ast::stmt::{FunStmt, Stmt};
use crate::interpreter::environment::Environment;
use crate::interpreter::value::{Callable, CallableFun, DefinedFunction, InterpreterValue, ValueKind};
use crate::frontend::lexer::token::TokenType;
use crate::frontend::parser::parser::{parse_expression, parse_program};
use crate::frontend::resolver::resolver_pass::resolve_variables;
use crate::infra::diagnostic::InterpreterDiagnostic;
use crate::infra::result::{FelicoResult};
use crate::infra::source_file::SourceFileHandle;
use std::ops::Deref;
use std::sync::Arc;
use crate::interpreter::core_definitions::get_core_definitions;

type PrintFn = Box<dyn Fn(&InterpreterValue) -> ()>;

pub struct Interpreter {
    source_file: SourceFileHandle,
    environment: Environment,
    pub(crate) print_fn: PrintFn,
    fuel: i64,
    available_stack: i64,
}

pub enum StmtResult {
    Continue,
    Return(InterpreterValue),
}

impl StmtResult {
    fn is_return(&self) -> bool {
        matches!(self, Self::Return(_))
    }
}

impl Interpreter {
    pub fn new(source_file: SourceFileHandle) -> FelicoResult<Self> {
        let mut environment = Environment::new();
        for core_definition in get_core_definitions() {
            environment.define(&core_definition.name, core_definition.value.clone());
        }

        environment.enter_new();
        Ok(Self {
            source_file,
            environment,
            print_fn: Box::new(|value|  {
                print!("{}", value);
            }),
            fuel: 1000,
            available_stack: 20,
        })
    }

    pub fn set_print_fn(&mut self, print_fn: PrintFn) {
        self.print_fn = print_fn;
    }

    pub fn set_fuel(&mut self, fuel: i64) {
        self.fuel = fuel;
    }

    fn spend_fuel<T: AstData>(&mut self, stmt: &AstNode<T>) -> FelicoResult<()> {
        self.fuel -= 1;
        if self.fuel <= 0 {
            return self.create_diagnostic(stmt, "Out of fuel! Execution took to many loops/function calls.", |diagnostic| {
                diagnostic.add_primary_label(&stmt.location);
            });
        }
        Ok(())
    }

    pub fn evaluate_program(mut self) -> FelicoResult<()> {
        let mut program = parse_program(self.source_file.clone())?;
        resolve_variables(&mut program)?;
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
        Ok(match expr.data.deref() {
            Expr::Literal(LiteralExpr::Unit) => {
                InterpreterValue::unit()
            }
            Expr::Literal(LiteralExpr::String(string)) => {
                InterpreterValue::new_string(string.clone())
            }
            Expr::Literal(LiteralExpr::Number(number)) => {
                InterpreterValue::f64(*number)
            }
            Expr::Literal(LiteralExpr::Bool(bool)) => {
                InterpreterValue::bool(*bool)
            }
            Expr::Unary(unary) => {
                let sub_expression = self.evaluate_expr(&unary.right)?;
                match unary.operator.token_type {
                    TokenType::Minus => {
                        match sub_expression.val {
                            ValueKind::Number(number) => {
                                InterpreterValue::f64(-number)
                            }
                            _ => {
                                return self.create_diagnostic(expr, format!("Value '{:?}' cannot be negated", sub_expression), |diagnostic| {
                                    diagnostic.add_primary_label(&expr.location);
                                });
                            }
                        }
                    }
                    _ => {
                        return self.create_diagnostic(expr, format!("Unsupported unary operator {}", unary.operator.token_type), |diagnostic| {
                            diagnostic.add_primary_label(&expr.location);
                        });
                    }
                }
            }
            Expr::Binary(binary) => {
                let left_value = self.evaluate_expr(&binary.left)?;
                // Handle "and" & "or" upfront to handle short-circuiting logic
                match binary.operator.token_type {
                    TokenType::Or | TokenType::And => {
                        if let ValueKind::Bool(left) = left_value.val {
                            if binary.operator.token_type == TokenType::Or {
                                if left {
                                    return Ok(InterpreterValue::bool(true));
                                }
                            } else { // AND
                                if !left {
                                    return Ok(InterpreterValue::bool(false));
                                }
                            }
                            let right_value = self.evaluate_expr(&binary.right)?;
                            return match right_value.val {
                                ValueKind::Bool(_) => {
                                    Ok(right_value)
                                } // Ok
                                _ => {
                                    self.create_diagnostic(expr, format!("Unsupported operand for boolean {} operation: {}", binary.operator.token_type, right_value), |diagnostic| {
                                        diagnostic.add_primary_label(&binary.right.location);
                                    })
                                }
                            }
                        } else {
                            return self.create_diagnostic(expr, format!("Unsupported operand for boolean {} operation: {}", binary.operator.token_type, left_value), |diagnostic| {
                                diagnostic.add_primary_label(&binary.left.location);
                            });
                        }
                    }
                    _ => {}
                };
                let right_value = self.evaluate_expr(&binary.right)?;
                match (left_value.val, right_value.val) {
                    (ValueKind::Number(left), ValueKind::Number(right)) => {
                        match binary.operator.token_type {
                            TokenType::Minus => {
                                InterpreterValue::f64(left - right)
                            }
                            TokenType::Plus => {
                                InterpreterValue::f64(left + right)
                            }
                            TokenType::Star => {
                                InterpreterValue::f64(left * right)
                            }
                            TokenType::Slash => {
                                InterpreterValue::f64(left / right)
                            }
                            TokenType::EqualEqual => {
                                InterpreterValue::bool(left == right)
                            }
                            TokenType::BangEqual => {
                                InterpreterValue::bool(left != right)
                            }
                            TokenType::Greater => {
                                InterpreterValue::bool(left > right)
                            }
                            TokenType::GreaterEqual => {
                                InterpreterValue::bool(left >= right)
                            }
                            TokenType::Less => {
                                InterpreterValue::bool(left < right)
                            }
                            TokenType::LessEqual => {
                                InterpreterValue::bool(left <= right)
                            }
                            _ => {
                                return self.create_diagnostic(expr, format!("Unsupported binary operator for numbers: {}", binary.operator.lexeme()), |diagnostic| {
                                    diagnostic.add_primary_label(&binary.operator.location);
                                });
                            }
                        }
                    }
                    (ValueKind::String(left), right) => {
                        match binary.operator.token_type {
                            TokenType::Plus => {
                                return Ok(InterpreterValue::new_string(left + &format!("{}", right)))
                            }
                            _ => {
                                return self.create_diagnostic(expr, format!("Unsupported binary operator for string: {}", binary.operator.lexeme()), |diagnostic| {
                                    diagnostic.add_primary_label(&binary.operator.location);
                                });
                            }
                        }
                    }
                    (left, right) => {
                        return self.create_diagnostic(expr, format!("Operator {:?} not defined for values {:?} and {:?}", binary.operator.token_type, left, right), |diagnostic| {
                            diagnostic.add_primary_label(&expr.location);
                        });
                    }
                }
                }
            Expr::Variable(var_use) => {
                self.environment.get_at_distance(var_use.variable.lexeme(), var_use.distance)?.clone()
            }
            Expr::Assign(assign) => {
                let value = self.evaluate_expr(&assign.value)?;
                self.environment.assign_at_distance(&assign.destination.lexeme(), assign.distance, value.clone())?;
                value
            }
            Expr::Get(_get) => {
                todo!("Get not supported");
                /*
                let object = self.evaluate_expr(&get.object)?;
                return if let InterpreterValue::Object(instance) = &object {
                    if let Some(value) = instance.borrow().fields.get(get.name.lexeme()) {
                        return Ok(value.clone());
                    }
                    if let Some(method) = instance.borrow().class.method_map.get(get.name.lexeme()) {
                        if let InterpreterValue::Callable(callable) = &method {
                            if let CallableFun::Defined(fun) = &*callable.fun {
                                let closure = fun.closure.child_environment();
                                closure.define("this", object.clone());
                                return Ok(InterpreterValue::Callable(Callable {
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
            Expr::Set(_set) => {
                todo!("Set not supported");
                /*
                let object = self.evaluate_expr(&set.object)?;
                return if let InterpreterValue::Object(instance) = object {
                    let value = self.evaluate_expr(&set.value)?;
                    instance.borrow_mut().fields.insert(set.name.lexeme().to_string(), value.clone());
                    Ok(value)
                } else {
                    self.create_diagnostic(expr, format!("Expected object for dot access instead found {:?}", &object), |diagnostic| {
                        diagnostic.add_primary_label(&expr.location);
                    })
                }*/
            }
            Expr::Call(call) => {
                self.available_stack -= 1;
                if self.available_stack <= 0 {
                    return self.create_diagnostic(expr, "Stack size exceeded.", |diagnostic| {
                        diagnostic.add_primary_label(&expr.location);
                    });
                }
                let callee = self.evaluate_expr(&call.callee)?;
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
                        arguments.push(self.evaluate_expr(expr)?);
                    }
                    let result = match &callable.fun.as_ref() {
                        CallableFun::Native(fun) => {
                            match fun(self, arguments) {
                                Ok(result) => {
                                    result
                                }
                                Err(err) => {
                                    return self.create_diagnostic(expr, format!("Error in native call to {}(): {}", callable.name, err), |diagnostic| {
                                        diagnostic.add_primary_label(&call.callee.location);
                                    });
                                }
                            }
                        }
                        CallableFun::Defined(defined_function) => {
                            let old_environment = self.environment.clone();
                            self.environment = defined_function.closure.clone().child_environment();
                            defined_function.fun_stmt.parameters.iter().zip(arguments.into_iter()).for_each(|(name, value)| {
                                self.environment.define(name.lexeme(), value);
                            });
                            let result = self.evaluate_stmt(&defined_function.fun_stmt.body)?;
                            self.environment = old_environment;
                            match result {
                                StmtResult::Continue => {
                                    InterpreterValue::unit()
                                }
                                StmtResult::Return(value) => {
                                    value
                                }
                            }
                        }
                    };
                    self.available_stack += 1;
                    result
                } else {
                    return self.create_diagnostic(expr, format!("Expression '{:?}' is not callable", callee), |diagnostic| {
                        diagnostic.add_primary_label(&call.callee.location);
                    });
                }
            }
        })
    }

    fn create_diagnostic<T, S: Into<String>, A: AstData>(&self, ast_node: &AstNode<A>, message: S,  mut f: impl FnMut(&mut InterpreterDiagnostic) -> ()) -> FelicoResult<T> {
        let mut diagnostic = InterpreterDiagnostic::new(&ast_node.location.source_file, message.into());
        f(&mut diagnostic);
        Err(diagnostic.into())
    }


    fn evaluate_stmt(&mut self, stmt: &AstNode<Stmt>) -> FelicoResult<StmtResult> {
        match stmt.data.deref() {
            Stmt::Expression(expr) => {
                self.evaluate_expr(&expr.expression)?;
            }
            Stmt::Let(var) => {
                let value = self.evaluate_expr(&var.expression)?;
                self.environment.define(&var.name.lexeme(), value);
            }
            Stmt::Fun(fun) => {
                let callable = self.create_fun_callable(fun);
                self.environment.define(fun.name.lexeme(), callable);
            }
            Stmt::Block(block) => {
                self.environment.enter_new();
                let result = self.evaluate_stmts(&block.stmts[..])?;
                self.environment.exit();
                return Ok(result);
            }
            Stmt::If(if_stmt) => {
                match self.evaluate_expr(&if_stmt.condition)?.val {
                    ValueKind::Bool(true) => {
                        let result = self.evaluate_stmt(&if_stmt.then_stmt)?;
                        if result.is_return() {
                            return Ok(result);
                        }
                    }
                    ValueKind::Bool(false) => {
                        if let Some(else_stmt) = &if_stmt.else_stmt {
                            let result = self.evaluate_stmt(else_stmt)?;
                            if result.is_return() {
                                return Ok(result);
                            }
                        }
                    }
                    other => {
                        return self.create_diagnostic(&if_stmt.condition, format!("Expected true or false in if condition, but found '{}' instead", other), |diagnostic| {
                            diagnostic.add_primary_label(&if_stmt.condition.location);
                        });
                    }
                }
            }
            Stmt::While(while_stmt) => {
                loop {
                    match self.evaluate_expr(&while_stmt.condition)?.val {
                        ValueKind::Bool(true) => {
                            let result = self.evaluate_stmt(&while_stmt.body_stmt)?;
                            if result.is_return() {
                                return Ok(result);
                            }
                        }
                        ValueKind::Bool(false) => {
                            break;
                        }
                        other => {
                            return self.create_diagnostic(&while_stmt.condition, format!("Expected true or false in loop condition, but found '{}' instead", other), |diagnostic| {
                                diagnostic.add_primary_label(&while_stmt.condition.location);
                            });
                        }
                    }
                    self.spend_fuel(&while_stmt.condition)?;
                }
            }
            Stmt::Return(return_stmt) => {
                let result = self.evaluate_expr(&return_stmt.expression)?;
                return Ok(StmtResult::Return(result));
            }
        }
        Ok(StmtResult::Continue)
    }

    fn create_fun_callable(&mut self, fun: &FunStmt) -> InterpreterValue {
        let callable = InterpreterValue::callable(Callable {
            name: fun.name.lexeme().to_string(),
            arity: fun.parameters.len(),
            fun: Arc::new(CallableFun::Defined(DefinedFunction {
                fun_stmt: fun.clone(),
                closure: self.environment.clone(),
            })),
        });
        callable
    }

    fn evaluate_stmts(&mut self, stmts: &[AstNode<Stmt>]) -> FelicoResult<StmtResult> {
        for stmt in stmts {
            let result = self.evaluate_stmt(stmt)?;
            if result.is_return() {
                return Ok(result);
            }
        }
        Ok(StmtResult::Continue)
    }
}

pub fn run_program_to_string(name: &str, input: &str) -> FelicoResult<String> {
    let mut interpreter = Interpreter::new(SourceFileHandle::from_string(name, input))?;
    let output_buffer = std::sync::Arc::new(std::sync::RwLock::new(String::new()));
    let output_buffer_clone = output_buffer.clone();
    interpreter.set_print_fn(Box::new(move |value| output_buffer.write().unwrap().push_str(&format!("{}", value))));
    interpreter.evaluate_program()?;
    let guard = output_buffer_clone.write().unwrap();
    Ok(guard.deref().clone())
}

#[cfg(test)]
mod tests {
    use crate::interpreter::eval::eval_expression;
    use crate::interpreter::interpreter::{run_program_to_string, Interpreter};
    use crate::infra::diagnostic::unwrap_diagnostic_to_string;
    use crate::infra::source_file::SourceFileHandle;
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
        literal_number_0: "0" => expect!["Number(0.0)"];
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
            fun fib(n) {
                 if (n <= 1) return n;
                 return fib(n - 2) + fib(n - 1);
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
        naked_if_condition: "debug_print(3+true);" => expect![[r#"
            × Operator Plus not defined for values Number(3.0) and Bool(true)
               ╭─[naked_if_condition:1:13]
             1 │ debug_print(3+true);
               ·             ───────
               ╰────"#]];
        call_uncallable: "true();" => expect![[r#"
            × Expression 'Bool(true)' is not callable
               ╭─[call_uncallable:1:1]
             1 │ true();
               · ────
               ╰────"#]];
        call_wrong_arity: "sqrt();" => expect![[r#"
            × Wrong number of arguments in function call 'sqrt' - Expected: 1, got: 0 instead
               ╭─[call_wrong_arity:1:1]
             1 │ sqrt();
               · ────
               ╰────"#]];
        call_wrong_arity_defined: "fun foo(a) {}\ndebug_print(3);\nfoo();" => expect![[r#"
            × Wrong number of arguments in function call 'foo' - Expected: 1, got: 0 instead
               ╭─[call_wrong_arity_defined:3:1]
             1 │ fun foo(a) {}
               ·     ─┬─
               ·      ╰── 'foo' defined here
             2 │ debug_print(3);
             3 │ foo();
               · ───
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
        wrong_type_in_for: "for(;3;) {}" => expect![[r#"
            × Expected true or false in loop condition, but found '3' instead
               ╭─[wrong_type_in_for:1:6]
             1 │ for(;3;) {}
               ·      ─
               ╰────"#]];
        sqrt_true: "sqrt(true);" => expect![[r#"
            × Error in native call to sqrt(): Expected number as argument to sqrt
               ╭─[sqrt_true:1:1]
             1 │ sqrt(true);
               · ────
               ╰────"#]];
        endless_loop: "while(true) {}" => expect![[r#"
            × Out of fuel! Execution took to many loops/function calls.
               ╭─[endless_loop:1:7]
             1 │ while(true) {}
               ·       ────
               ╰────"#]];
        endless_for: "for(;true;) {}" => expect![[r#"
            × Out of fuel! Execution took to many loops/function calls.
               ╭─[endless_for:1:6]
             1 │ for(;true;) {}
               ·      ────
               ╰────"#]];
        endless_recursion: "fun a() {a();} a();" => expect![[r#"
            × Stack size exceeded.
               ╭─[endless_recursion:1:11]
             1 │ fun a() {a();} a();
               ·           ───
               ╰────"#]];
    );
}