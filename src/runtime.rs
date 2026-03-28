use std::collections::HashMap;
use std::fmt;

use crate::ast::{BinaryOp, Expr, MatchArm, Pattern, Stmt, UnaryOp};
use crate::error::{EiriadError, EiriadResult};

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    Request(HttpRequestValue),
    Response(HttpResponseValue),
    Some(Box<Value>),
    None,
    Ok(Box<Value>),
    Err(Box<Value>),
    Function(UserFunction),
    Unit,
}

#[derive(Debug, Clone)]
pub struct UserFunction {
    params: Vec<String>,
    body: Expr,
    captured: HashMap<String, Binding>,
}

#[derive(Debug, Clone)]
pub struct HttpRequestValue {
    method: String,
    path: String,
    body: String,
}

#[derive(Debug, Clone)]
pub struct HttpResponseValue {
    status: u16,
    body: String,
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(v) => write!(f, "{}", v),
            Value::Float(v) => write!(f, "{}", v),
            Value::Bool(v) => write!(f, "{}", v),
            Value::Str(v) => write!(f, "{}", v),
            Value::Request(v) => write!(f, "Request({}, {})", v.method, v.path),
            Value::Response(v) => write!(f, "Response({}, {})", v.status, v.body),
            Value::Some(v) => write!(f, "Some({})", v),
            Value::None => write!(f, "None"),
            Value::Ok(v) => write!(f, "Ok({})", v),
            Value::Err(v) => write!(f, "Err({})", v),
            Value::Function(_) => write!(f, "<fn>"),
            Value::Unit => write!(f, "()"),
        }
    }
}

#[derive(Debug, Clone)]
struct Binding {
    mutable: bool,
    value: Value,
}

#[derive(Debug, Default)]
pub struct Runtime {
    env: HashMap<String, Binding>,
    output: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ExecResult {
    pub last_value: Value,
    pub output: Vec<String>,
}

impl Runtime {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.env.clear();
        self.output.clear();
    }

    pub fn exec_program(&mut self, stmts: &[Stmt]) -> EiriadResult<ExecResult> {
        self.output.clear();
        let mut last = Value::Unit;
        for stmt in stmts {
            last = self.eval_stmt(stmt)?;
        }
        Ok(ExecResult {
            last_value: last,
            output: self.output.clone(),
        })
    }

    pub fn snapshot_env(&self) -> Vec<(String, String)> {
        let mut entries: Vec<(String, String)> = self
            .env
            .iter()
            .map(|(name, binding)| {
                let mutability = if binding.mutable { "mut" } else { "let" };
                (
                    name.clone(),
                    format!("{} = {} ({})", name, binding.value, mutability),
                )
            })
            .collect();
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        entries
    }

    fn eval_stmt(&mut self, stmt: &Stmt) -> EiriadResult<Value> {
        match stmt {
            Stmt::Let {
                name,
                mutable,
                expr,
            } => {
                let value = self.eval_expr(expr)?;
                self.env.insert(
                    name.clone(),
                    Binding {
                        mutable: *mutable,
                        value: value.clone(),
                    },
                );
                Ok(value)
            }
            Stmt::Fn { name, params, body } => {
                let func = Value::Function(UserFunction {
                    params: params.clone(),
                    body: body.clone(),
                    captured: self.env.clone(),
                });
                self.env.insert(
                    name.clone(),
                    Binding {
                        mutable: false,
                        value: func.clone(),
                    },
                );
                Ok(func)
            }
            Stmt::Assign { name, expr } => {
                let value = self.eval_expr(expr)?;
                let binding = self
                    .env
                    .get_mut(name)
                    .ok_or_else(|| EiriadError::new(format!("Unknown variable '{}'", name)))?;
                if !binding.mutable {
                    return Err(EiriadError::new(format!(
                        "Cannot assign to immutable binding '{}'",
                        name
                    )));
                }
                binding.value = value.clone();
                Ok(value)
            }
            Stmt::Expr(expr) => self.eval_expr(expr),
        }
    }

    fn eval_expr(&mut self, expr: &Expr) -> EiriadResult<Value> {
        match expr {
            Expr::Int(v) => Ok(Value::Int(*v)),
            Expr::Float(v) => Ok(Value::Float(*v)),
            Expr::Bool(v) => Ok(Value::Bool(*v)),
            Expr::Str(v) => Ok(Value::Str(v.clone())),
            Expr::None => Ok(Value::None),
            Expr::Ident(name) => self
                .env
                .get(name)
                .map(|b| b.value.clone())
                .ok_or_else(|| EiriadError::new(format!("Unknown variable '{}'", name))),
            Expr::Unary { op, expr } => {
                let v = self.eval_expr(expr)?;
                self.eval_unary(*op, v)
            }
            Expr::Binary { left, op, right } => {
                if *op == BinaryOp::And {
                    let lv = self.eval_expr(left)?;
                    if !truthy(&lv) {
                        return Ok(Value::Bool(false));
                    }
                    let rv = self.eval_expr(right)?;
                    return Ok(Value::Bool(truthy(&rv)));
                }
                if *op == BinaryOp::Or {
                    let lv = self.eval_expr(left)?;
                    if truthy(&lv) {
                        return Ok(Value::Bool(true));
                    }
                    let rv = self.eval_expr(right)?;
                    return Ok(Value::Bool(truthy(&rv)));
                }

                let lv = self.eval_expr(left)?;
                let rv = self.eval_expr(right)?;
                self.eval_binary(*op, lv, rv)
            }
            Expr::Lambda { params, body } => Ok(Value::Function(UserFunction {
                params: params.clone(),
                body: (*body.clone()),
                captured: self.env.clone(),
            })),
            Expr::Call { callee, args } => {
                let mut values = Vec::with_capacity(args.len());
                for arg in args {
                    values.push(self.eval_expr(arg)?);
                }

                if let Expr::Ident(name) = callee.as_ref() {
                    if let Some(binding) = self.env.get(name) {
                        if let Value::Function(func) = &binding.value {
                            let func = func.clone();
                            return self.call_user_function(&func, &values);
                        }
                    }

                    return self.call_builtin(name, &values);
                }

                match self.eval_expr(callee)? {
                    Value::Function(func) => self.call_user_function(&func, &values),
                    _ => Err(EiriadError::new("Expression is not callable")),
                }
            }
            Expr::Match { value, arms } => {
                let matched = self.eval_expr(value)?;
                self.eval_match_arms(&matched, arms)
            }
        }
    }

    fn eval_match_arms(&mut self, value: &Value, arms: &[MatchArm]) -> EiriadResult<Value> {
        for arm in arms {
            if let Some(bindings) = pattern_bindings(&arm.pattern, value) {
                let saved_env = self.env.clone();
                for (name, bound) in bindings {
                    self.env.insert(
                        name,
                        Binding {
                            mutable: false,
                            value: bound,
                        },
                    );
                }
                let out = self.eval_expr(&arm.expr);
                self.env = saved_env;
                return out;
            }
        }
        Err(EiriadError::new("No match arm matched the value"))
    }

    fn call_user_function(&mut self, func: &UserFunction, args: &[Value]) -> EiriadResult<Value> {
        if args.len() != func.params.len() {
            return Err(EiriadError::new(format!(
                "Function expects {} argument(s), got {}",
                func.params.len(),
                args.len()
            )));
        }

        let saved_env = self.env.clone();
        self.env = func.captured.clone();
        for (param, arg) in func.params.iter().zip(args.iter()) {
            self.env.insert(
                param.clone(),
                Binding {
                    mutable: false,
                    value: arg.clone(),
                },
            );
        }

        let out = self.eval_expr(&func.body);
        self.env = saved_env;
        out
    }

    fn eval_unary(&self, op: UnaryOp, v: Value) -> EiriadResult<Value> {
        match op {
            UnaryOp::Neg => match v {
                Value::Int(n) => Ok(Value::Int(-n)),
                Value::Float(n) => Ok(Value::Float(-n)),
                _ => Err(EiriadError::new("Unary '-' expects Int or Float")),
            },
            UnaryOp::Not => Ok(Value::Bool(!truthy(&v))),
        }
    }

    fn eval_binary(&self, op: BinaryOp, left: Value, right: Value) -> EiriadResult<Value> {
        match op {
            BinaryOp::Add => add(left, right),
            BinaryOp::Sub => numeric_bin(left, right, |a, b| a - b, |a, b| a - b),
            BinaryOp::Mul => numeric_bin(left, right, |a, b| a * b, |a, b| a * b),
            BinaryOp::Div => div(left, right),
            BinaryOp::Mod => int_bin(left, right, |a, b| a % b),
            BinaryOp::Pow => pow(left, right),
            BinaryOp::Eq => Ok(Value::Bool(equals(&left, &right))),
            BinaryOp::Ne => Ok(Value::Bool(!equals(&left, &right))),
            BinaryOp::Lt => cmp(left, right, |o| o < 0),
            BinaryOp::Le => cmp(left, right, |o| o <= 0),
            BinaryOp::Gt => cmp(left, right, |o| o > 0),
            BinaryOp::Ge => cmp(left, right, |o| o >= 0),
            BinaryOp::And | BinaryOp::Or | BinaryOp::Pipe => unreachable!(),
        }
    }

    fn call_builtin(&mut self, name: &str, args: &[Value]) -> EiriadResult<Value> {
        match name {
            "print" => {
                if args.len() != 1 {
                    return Err(EiriadError::new("print expects exactly 1 argument"));
                }
                self.output.push(args[0].to_string());
                Ok(Value::Unit)
            }
            "len" => {
                if args.len() != 1 {
                    return Err(EiriadError::new("len expects exactly 1 argument"));
                }
                match &args[0] {
                    Value::Str(s) => Ok(Value::Int(s.chars().count() as i64)),
                    _ => Err(EiriadError::new("len currently supports Str only")),
                }
            }
            "sqrt" => {
                if args.len() != 1 {
                    return Err(EiriadError::new("sqrt expects exactly 1 argument"));
                }
                let value = to_f64(&args[0])?;
                Ok(Value::Float(value.sqrt()))
            }
            "typeof" => {
                if args.len() != 1 {
                    return Err(EiriadError::new("typeof expects exactly 1 argument"));
                }
                let kind = match args[0] {
                    Value::Int(_) => "Int",
                    Value::Float(_) => "Float",
                    Value::Bool(_) => "Bool",
                    Value::Str(_) => "Str",
                    Value::Request(_) => "Request",
                    Value::Response(_) => "Response",
                    Value::Some(_) | Value::None => "Option",
                    Value::Ok(_) | Value::Err(_) => "Result",
                    Value::Function(_) => "fn",
                    Value::Unit => "()",
                };
                Ok(Value::Str(kind.to_string()))
            }
            "serve" => {
                if args.len() != 2 {
                    return Err(EiriadError::new("serve expects exactly 2 arguments"));
                }
                let port = match args[0] {
                    Value::Int(v) if (1..=65535).contains(&v) => v as u16,
                    Value::Int(_) => {
                        return Err(EiriadError::new("serve port must be between 1 and 65535"));
                    }
                    _ => return Err(EiriadError::new("serve expects Int port as first argument")),
                };

                let handler = match &args[1] {
                    Value::Function(func) => func.clone(),
                    _ => {
                        return Err(EiriadError::new(
                            "serve expects function(request) as second argument",
                        ));
                    }
                };

                eiriad_serve(self, port, &handler)
            }
            "request_method" => {
                if args.len() != 1 {
                    return Err(EiriadError::new("request_method expects exactly 1 argument"));
                }
                match &args[0] {
                    Value::Request(req) => Ok(Value::Str(req.method.clone())),
                    _ => Err(EiriadError::new("request_method expects Request value")),
                }
            }
            "request_path" => {
                if args.len() != 1 {
                    return Err(EiriadError::new("request_path expects exactly 1 argument"));
                }
                match &args[0] {
                    Value::Request(req) => Ok(Value::Str(req.path.clone())),
                    _ => Err(EiriadError::new("request_path expects Request value")),
                }
            }
            "request_body" => {
                if args.len() != 1 {
                    return Err(EiriadError::new("request_body expects exactly 1 argument"));
                }
                match &args[0] {
                    Value::Request(req) => Ok(Value::Str(req.body.clone())),
                    _ => Err(EiriadError::new("request_body expects Request value")),
                }
            }
            "response" => {
                if args.len() != 2 {
                    return Err(EiriadError::new("response expects exactly 2 arguments"));
                }

                let status = match args[0] {
                    Value::Int(v) if (100..=599).contains(&v) => v as u16,
                    Value::Int(_) => {
                        return Err(EiriadError::new("response status must be between 100 and 599"));
                    }
                    _ => {
                        return Err(EiriadError::new(
                            "response expects Int status as first argument",
                        ));
                    }
                };

                let body = match &args[1] {
                    Value::Str(s) => s.clone(),
                    _ => {
                        return Err(EiriadError::new(
                            "response expects Str body as second argument",
                        ));
                    }
                };

                Ok(Value::Response(HttpResponseValue { status, body }))
            }
            "fetch" | "http_get" => {
                if args.len() != 1 {
                    return Err(EiriadError::new(format!(
                        "{} expects exactly 1 argument",
                        name
                    )));
                }
                let url = match &args[0] {
                    Value::Str(s) => s,
                    _ => return Err(EiriadError::new(format!("{} expects Str URL", name))),
                };
                eiriad_http_request("GET", url, None)
            }
            "http_delete" | "http_head" | "http_options" => {
                if args.len() != 1 {
                    return Err(EiriadError::new(format!(
                        "{} expects exactly 1 argument",
                        name
                    )));
                }
                let url = match &args[0] {
                    Value::Str(s) => s,
                    _ => return Err(EiriadError::new(format!("{} expects Str URL", name))),
                };
                let method = match name {
                    "http_delete" => "DELETE",
                    "http_head" => "HEAD",
                    _ => "OPTIONS",
                };
                eiriad_http_request(method, url, None)
            }
            "http_post" | "http_put" | "http_patch" => {
                if args.len() != 2 {
                    return Err(EiriadError::new(format!(
                        "{} expects exactly 2 arguments",
                        name
                    )));
                }
                let url = match &args[0] {
                    Value::Str(s) => s,
                    _ => return Err(EiriadError::new(format!("{} expects Str URL", name))),
                };
                let body = match &args[1] {
                    Value::Str(s) => s,
                    _ => return Err(EiriadError::new(format!("{} expects Str body", name))),
                };
                let method = match name {
                    "http_post" => "POST",
                    "http_put" => "PUT",
                    _ => "PATCH",
                };
                eiriad_http_request(method, url, Some(body))
            }
            "Some" => {
                if args.len() != 1 {
                    return Err(EiriadError::new("Some expects exactly 1 argument"));
                }
                Ok(Value::Some(Box::new(args[0].clone())))
            }
            "Ok" => {
                if args.len() != 1 {
                    return Err(EiriadError::new("Ok expects exactly 1 argument"));
                }
                Ok(Value::Ok(Box::new(args[0].clone())))
            }
            "Err" => {
                if args.len() != 1 {
                    return Err(EiriadError::new("Err expects exactly 1 argument"));
                }
                Ok(Value::Err(Box::new(args[0].clone())))
            }
            "unwrap_or" => {
                if args.len() != 2 {
                    return Err(EiriadError::new("unwrap_or expects exactly 2 arguments"));
                }
                match &args[0] {
                    Value::Some(v) => Ok((**v).clone()),
                    Value::None => Ok(args[1].clone()),
                    Value::Ok(v) => Ok((**v).clone()),
                    Value::Err(_) => Ok(args[1].clone()),
                    _ => Err(EiriadError::new("unwrap_or expects Option or Result value")),
                }
            }
            "is_some" => {
                if args.len() != 1 {
                    return Err(EiriadError::new("is_some expects exactly 1 argument"));
                }
                Ok(Value::Bool(matches!(args[0], Value::Some(_))))
            }
            "is_none" => {
                if args.len() != 1 {
                    return Err(EiriadError::new("is_none expects exactly 1 argument"));
                }
                Ok(Value::Bool(matches!(args[0], Value::None)))
            }
            "is_ok" => {
                if args.len() != 1 {
                    return Err(EiriadError::new("is_ok expects exactly 1 argument"));
                }
                Ok(Value::Bool(matches!(args[0], Value::Ok(_))))
            }
            "is_err" => {
                if args.len() != 1 {
                    return Err(EiriadError::new("is_err expects exactly 1 argument"));
                }
                Ok(Value::Bool(matches!(args[0], Value::Err(_))))
            }
            _ => Err(EiriadError::new(format!("Unknown function '{}'", name))),
        }
    }
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Bool(b) => *b,
        Value::Int(n) => *n != 0,
        Value::Float(n) => *n != 0.0,
        Value::Str(s) => !s.is_empty(),
        Value::Request(_) => true,
        Value::Response(_) => true,
        Value::Some(_) => true,
        Value::None => false,
        Value::Ok(_) => true,
        Value::Err(_) => false,
        Value::Function(_) => true,
        Value::Unit => false,
    }
}

fn equals(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => x == y,
        (Value::Float(x), Value::Float(y)) => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Str(x), Value::Str(y)) => x == y,
        (Value::Request(x), Value::Request(y)) => {
            x.method == y.method && x.path == y.path && x.body == y.body
        }
        (Value::Response(x), Value::Response(y)) => x.status == y.status && x.body == y.body,
        (Value::Some(x), Value::Some(y)) => equals(x, y),
        (Value::None, Value::None) => true,
        (Value::Ok(x), Value::Ok(y)) => equals(x, y),
        (Value::Err(x), Value::Err(y)) => equals(x, y),
        (Value::Function(_), Value::Function(_)) => false,
        (Value::Unit, Value::Unit) => true,
        _ => false,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn eiriad_serve(runtime: &mut Runtime, port: u16, handler: &UserFunction) -> EiriadResult<Value> {
    use tiny_http::{Response, Server, StatusCode};

    let addr = format!("0.0.0.0:{}", port);
    let server = Server::http(&addr)
        .map_err(|e| EiriadError::new(format!("serve failed to bind {}: {}", addr, e)))?;

    for mut req in server.incoming_requests() {
        let mut body = String::new();
        if let Err(e) = req.as_reader().read_to_string(&mut body) {
            body = format!("<non-utf8 body: {}>", e);
        }

        let req_value = Value::Request(HttpRequestValue {
            method: req.method().as_str().to_string(),
            path: req.url().to_string(),
            body,
        });

        let (status, response_body) = match runtime.call_user_function(handler, &[req_value]) {
            Ok(Value::Response(resp)) => (resp.status, resp.body),
            Ok(Value::Str(body)) => (200u16, body),
            Ok(Value::Unit) => (204u16, String::new()),
            Ok(other) => (
                500u16,
                format!(
                    "handler must return response(status, body), Str, or Unit; got {}",
                    other
                ),
            ),
            Err(e) => (500u16, format!("handler error: {}", e)),
        };

        let response = Response::from_string(response_body).with_status_code(StatusCode(status));
        let _ = req.respond(response);
    }

    Ok(Value::Unit)
}

#[cfg(target_arch = "wasm32")]
fn eiriad_serve(
    _runtime: &mut Runtime,
    _port: u16,
    _handler: &UserFunction,
) -> EiriadResult<Value> {
    Err(EiriadError::new(
        "serve is not available in wasm/browser runtime",
    ))
}

fn to_f64(v: &Value) -> EiriadResult<f64> {
    match v {
        Value::Int(n) => Ok(*n as f64),
        Value::Float(n) => Ok(*n),
        _ => Err(EiriadError::new("Expected numeric value")),
    }
}

fn pattern_bindings(pattern: &Pattern, value: &Value) -> Option<Vec<(String, Value)>> {
    match pattern {
        Pattern::Wildcard => Some(Vec::new()),
        Pattern::Ident(name) => Some(vec![(name.clone(), value.clone())]),
        Pattern::None => matches!(value, Value::None).then_some(Vec::new()),
        Pattern::Some(inner) => {
            if let Value::Some(v) = value {
                pattern_bindings(inner, v)
            } else {
                None
            }
        }
        Pattern::Ok(inner) => {
            if let Value::Ok(v) = value {
                pattern_bindings(inner, v)
            } else {
                None
            }
        }
        Pattern::Err(inner) => {
            if let Value::Err(v) = value {
                pattern_bindings(inner, v)
            } else {
                None
            }
        }
    }
}

fn add(left: Value, right: Value) -> EiriadResult<Value> {
    match (left, right) {
        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
        (Value::Int(a), Value::Float(b)) => Ok(Value::Float((a as f64) + b)),
        (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a + (b as f64))),
        (Value::Str(a), Value::Str(b)) => Ok(Value::Str(format!("{}{}", a, b))),
        _ => Err(EiriadError::new("'+' expects numbers or strings")),
    }
}

fn div(left: Value, right: Value) -> EiriadResult<Value> {
    let denom = to_f64(&right)?;
    if denom == 0.0 {
        return Err(EiriadError::new("Division by zero"));
    }
    let num = to_f64(&left)?;
    Ok(Value::Float(num / denom))
}

fn pow(left: Value, right: Value) -> EiriadResult<Value> {
    let a = to_f64(&left)?;
    let b = to_f64(&right)?;
    Ok(Value::Float(a.powf(b)))
}

fn int_bin(left: Value, right: Value, op: impl Fn(i64, i64) -> i64) -> EiriadResult<Value> {
    match (left, right) {
        (Value::Int(a), Value::Int(b)) => {
            if b == 0 {
                return Err(EiriadError::new("Division by zero"));
            }
            Ok(Value::Int(op(a, b)))
        }
        _ => Err(EiriadError::new("Operator expects Int values")),
    }
}

fn numeric_bin(
    left: Value,
    right: Value,
    int_op: impl Fn(i64, i64) -> i64,
    float_op: impl Fn(f64, f64) -> f64,
) -> EiriadResult<Value> {
    match (left, right) {
        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(int_op(a, b))),
        (a, b) => Ok(Value::Float(float_op(to_f64(&a)?, to_f64(&b)?))),
    }
}

fn cmp(left: Value, right: Value, pred: impl Fn(i8) -> bool) -> EiriadResult<Value> {
    let out = match (left, right) {
        (Value::Int(a), Value::Int(b)) => ord_to_i8(a.cmp(&b)),
        (Value::Float(a), Value::Float(b)) => ord_to_i8(a.total_cmp(&b)),
        (Value::Int(a), Value::Float(b)) => ord_to_i8((a as f64).total_cmp(&b)),
        (Value::Float(a), Value::Int(b)) => ord_to_i8(a.total_cmp(&(b as f64))),
        (Value::Str(a), Value::Str(b)) => ord_to_i8(a.cmp(&b)),
        _ => return Err(EiriadError::new("Cannot compare these values")),
    };
    Ok(Value::Bool(pred(out)))
}

fn ord_to_i8(ord: std::cmp::Ordering) -> i8 {
    match ord {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn eiriad_http_request(method: &str, url: &str, body: Option<&str>) -> EiriadResult<Value> {
    let method = reqwest::Method::from_bytes(method.as_bytes())
        .map_err(|e| EiriadError::new(format!("invalid HTTP method: {}", e)))?;

    let client = reqwest::blocking::Client::new();
    let mut request = client.request(method, url);
    if let Some(body) = body {
        request = request.body(body.to_string());
    }

    let response = request
        .send()
        .map_err(|e| EiriadError::new(format!("HTTP request failed: {}", e)))?;

    let status = response.status();
    let body = response
        .text()
        .map_err(|e| EiriadError::new(format!("HTTP response read failed: {}", e)))?;

    if status.is_success() {
        Ok(Value::Ok(Box::new(Value::Str(body))))
    } else {
        Ok(Value::Err(Box::new(Value::Str(format!(
            "HTTP {}: {}",
            status.as_u16(),
            body
        )))))
    }
}

#[cfg(target_arch = "wasm32")]
fn eiriad_http_request(method: &str, url: &str, body: Option<&str>) -> EiriadResult<Value> {
    use web_sys::XmlHttpRequest;

    let xhr = XmlHttpRequest::new()
        .map_err(|e| EiriadError::new(format!("XMLHttpRequest init failed: {:?}", e)))?;

    xhr.open_with_async(method, url, false)
        .map_err(|e| EiriadError::new(format!("XMLHttpRequest open failed: {:?}", e)))?;

    if body.is_some() {
        xhr.set_request_header("Content-Type", "application/json")
            .map_err(|e| EiriadError::new(format!("set_request_header failed: {:?}", e)))?;
    }

    match body {
        Some(payload) => xhr
            .send_with_opt_str(Some(payload))
            .map_err(|e| EiriadError::new(format!("XMLHttpRequest send failed: {:?}", e)))?,
        None => xhr
            .send()
            .map_err(|e| EiriadError::new(format!("XMLHttpRequest send failed: {:?}", e)))?,
    }

    let status = xhr
        .status()
        .map_err(|e| EiriadError::new(format!("XMLHttpRequest status failed: {:?}", e)))?;
    let text = xhr
        .response_text()
        .map_err(|e| EiriadError::new(format!("XMLHttpRequest response read failed: {:?}", e)))?
        .unwrap_or_default();

    if (200..300).contains(&status) {
        Ok(Value::Ok(Box::new(Value::Str(text))))
    } else {
        Ok(Value::Err(Box::new(Value::Str(format!(
            "HTTP {}: {}",
            status, text
        )))))
    }
}
