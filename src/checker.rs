use std::collections::HashMap;

use crate::ast::{BinaryOp, Expr, MatchArm, Pattern, Stmt, UnaryOp};
use crate::error::{EiriadError, EiriadResult};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Type {
    Int,
    Float,
    Bool,
    Str,
    Unit,
    Fn(usize),
    Option(Box<Type>),
    Result(Box<Type>, Box<Type>),
    Unknown,
}

#[derive(Debug, Clone)]
struct Binding {
    mutable: bool,
    ty: Type,
}

#[derive(Debug, Default)]
pub struct Checker {
    env: HashMap<String, Binding>,
}

impl Checker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn check_program(&mut self, stmts: &[Stmt]) -> EiriadResult<()> {
        for stmt in stmts {
            self.check_stmt(stmt)?;
        }
        Ok(())
    }

    fn check_stmt(&mut self, stmt: &Stmt) -> EiriadResult<()> {
        match stmt {
            Stmt::Let {
                name,
                mutable,
                expr,
            } => {
                let ty = self.check_expr(expr)?;
                self.env.insert(
                    name.clone(),
                    Binding {
                        mutable: *mutable,
                        ty,
                    },
                );
                Ok(())
            }
            Stmt::Fn { name, params, body } => {
                // Predeclare for self-recursion and forward checks.
                self.env.insert(
                    name.clone(),
                    Binding {
                        mutable: false,
                        ty: Type::Fn(params.len()),
                    },
                );

                let saved_env = self.env.clone();
                for param in params {
                    self.env.insert(
                        param.clone(),
                        Binding {
                            mutable: false,
                            ty: Type::Unknown,
                        },
                    );
                }
                self.check_expr(body)?;
                self.env = saved_env;
                Ok(())
            }
            Stmt::Assign { name, expr } => {
                let rhs_ty = self.check_expr(expr)?;
                let binding = self
                    .env
                    .get(name)
                    .ok_or_else(|| EiriadError::new(format!("Unknown variable '{}'", name)))?;
                if !binding.mutable {
                    return Err(EiriadError::new(format!(
                        "Cannot assign to immutable binding '{}'",
                        name
                    )));
                }
                if !compatible(&binding.ty, &rhs_ty) {
                    return Err(EiriadError::new(format!(
                        "Type mismatch in assignment to '{}': expected {}, found {}",
                        name,
                        type_name(&binding.ty),
                        type_name(&rhs_ty)
                    )));
                }
                Ok(())
            }
            Stmt::Expr(expr) => {
                self.check_expr(expr)?;
                Ok(())
            }
        }
    }

    fn check_expr(&mut self, expr: &Expr) -> EiriadResult<Type> {
        match expr {
            Expr::Int(_) => Ok(Type::Int),
            Expr::Float(_) => Ok(Type::Float),
            Expr::Bool(_) => Ok(Type::Bool),
            Expr::Str(_) => Ok(Type::Str),
            Expr::None => Ok(Type::Option(Box::new(Type::Unknown))),
            Expr::Ident(name) => self
                .env
                .get(name)
                .map(|b| b.ty.clone())
                .ok_or_else(|| EiriadError::new(format!("Unknown variable '{}'", name))),
            Expr::Unary { op, expr } => {
                let inner = self.check_expr(expr)?;
                match op {
                    UnaryOp::Neg => {
                        if is_numeric(&inner) {
                            Ok(inner)
                        } else {
                            Err(EiriadError::new("Unary '-' expects numeric operand"))
                        }
                    }
                    UnaryOp::Not => {
                        if inner == Type::Bool {
                            Ok(Type::Bool)
                        } else {
                            Err(EiriadError::new("Unary '!' expects Bool operand"))
                        }
                    }
                }
            }
            Expr::Binary { left, op, right } => {
                let lt = self.check_expr(left)?;
                let rt = self.check_expr(right)?;
                self.check_binary(*op, &lt, &rt)
            }
            Expr::Lambda { params, body } => {
                let saved_env = self.env.clone();
                for param in params {
                    self.env.insert(
                        param.clone(),
                        Binding {
                            mutable: false,
                            ty: Type::Unknown,
                        },
                    );
                }
                self.check_expr(body)?;
                self.env = saved_env;
                Ok(Type::Fn(params.len()))
            }
            Expr::Call { callee, args } => {
                let mut arg_types = Vec::with_capacity(args.len());
                for arg in args {
                    arg_types.push(self.check_expr(arg)?);
                }

                if let Expr::Ident(name) = callee.as_ref() {
                    if let Some(binding) = self.env.get(name) {
                        match binding.ty {
                            Type::Fn(arity) => {
                                expect_arg_count(name, &arg_types, arity)?;
                                return Ok(Type::Unknown);
                            }
                            _ => {
                                return Err(EiriadError::new(format!("'{}' is not callable", name)))
                            }
                        }
                    }

                    return self.check_builtin(name, &arg_types);
                }

                match self.check_expr(callee)? {
                    Type::Fn(arity) => {
                        expect_arg_count("lambda", &arg_types, arity)?;
                        Ok(Type::Unknown)
                    }
                    _ => Err(EiriadError::new("Expression is not callable")),
                }
            }
            Expr::Match { value, arms } => {
                let value_ty = self.check_expr(value)?;
                self.check_match_arms(&value_ty, arms)
            }
        }
    }

    fn check_match_arms(&mut self, value_ty: &Type, arms: &[MatchArm]) -> EiriadResult<Type> {
        let mut arm_ty: Option<Type> = None;

        for arm in arms {
            let saved_env = self.env.clone();
            self.bind_pattern(&arm.pattern, value_ty)?;
            let ty = self.check_expr(&arm.expr)?;
            self.env = saved_env;

            if let Some(expected) = &arm_ty {
                if !compatible(expected, &ty) {
                    return Err(EiriadError::new(
                        "All match arms must return compatible types",
                    ));
                }
            } else {
                arm_ty = Some(ty);
            }
        }

        Ok(arm_ty.unwrap_or(Type::Unknown))
    }

    fn bind_pattern(&mut self, pattern: &Pattern, value_ty: &Type) -> EiriadResult<()> {
        match pattern {
            Pattern::Wildcard => Ok(()),
            Pattern::Ident(name) => {
                self.env.insert(
                    name.clone(),
                    Binding {
                        mutable: false,
                        ty: value_ty.clone(),
                    },
                );
                Ok(())
            }
            Pattern::None => match value_ty {
                Type::Option(_) | Type::Unknown => Ok(()),
                _ => Err(EiriadError::new("Pattern None expects Option value")),
            },
            Pattern::Some(inner) => match value_ty {
                Type::Option(inner_ty) => self.bind_pattern(inner, inner_ty),
                Type::Unknown => self.bind_pattern(inner, &Type::Unknown),
                _ => Err(EiriadError::new("Pattern Some(_) expects Option value")),
            },
            Pattern::Ok(inner) => match value_ty {
                Type::Result(ok_ty, _) => self.bind_pattern(inner, ok_ty),
                Type::Unknown => self.bind_pattern(inner, &Type::Unknown),
                _ => Err(EiriadError::new("Pattern Ok(_) expects Result value")),
            },
            Pattern::Err(inner) => match value_ty {
                Type::Result(_, err_ty) => self.bind_pattern(inner, err_ty),
                Type::Unknown => self.bind_pattern(inner, &Type::Unknown),
                _ => Err(EiriadError::new("Pattern Err(_) expects Result value")),
            },
        }
    }

    fn check_binary(&self, op: BinaryOp, left: &Type, right: &Type) -> EiriadResult<Type> {
        match op {
            BinaryOp::Add => {
                if left == &Type::Unknown || right == &Type::Unknown {
                    return Ok(Type::Unknown);
                }
                if is_numeric(left) && is_numeric(right) {
                    if left == &Type::Float || right == &Type::Float {
                        Ok(Type::Float)
                    } else {
                        Ok(Type::Int)
                    }
                } else if left == &Type::Str && right == &Type::Str {
                    Ok(Type::Str)
                } else {
                    Err(EiriadError::new(
                        "'+' expects two numeric operands or two Str operands",
                    ))
                }
            }
            BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Mod | BinaryOp::Pow => {
                if left == &Type::Unknown || right == &Type::Unknown {
                    return Ok(Type::Unknown);
                }
                if is_numeric(left) && is_numeric(right) {
                    if op == BinaryOp::Mod {
                        if left == &Type::Int && right == &Type::Int {
                            Ok(Type::Int)
                        } else {
                            Err(EiriadError::new("'%' expects Int operands"))
                        }
                    } else if left == &Type::Float || right == &Type::Float {
                        Ok(Type::Float)
                    } else {
                        Ok(Type::Int)
                    }
                } else {
                    Err(EiriadError::new(
                        "Numeric operator expects numeric operands",
                    ))
                }
            }
            BinaryOp::Div => {
                if left == &Type::Unknown || right == &Type::Unknown {
                    return Ok(Type::Unknown);
                }
                if is_numeric(left) && is_numeric(right) {
                    Ok(Type::Float)
                } else {
                    Err(EiriadError::new("'/' expects numeric operands"))
                }
            }
            BinaryOp::Eq | BinaryOp::Ne => {
                if left == &Type::Unknown || right == &Type::Unknown {
                    return Ok(Type::Bool);
                }
                if comparable(left, right) {
                    Ok(Type::Bool)
                } else {
                    Err(EiriadError::new("'=='/'!=' operands are not comparable"))
                }
            }
            BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => {
                if left == &Type::Unknown || right == &Type::Unknown {
                    return Ok(Type::Bool);
                }
                if ordered(left, right) {
                    Ok(Type::Bool)
                } else {
                    Err(EiriadError::new(
                        "Comparison expects compatible ordered operands",
                    ))
                }
            }
            BinaryOp::And | BinaryOp::Or => {
                if left == &Type::Unknown || right == &Type::Unknown {
                    return Ok(Type::Bool);
                }
                if left == &Type::Bool && right == &Type::Bool {
                    Ok(Type::Bool)
                } else {
                    Err(EiriadError::new("'&&'/'||' expects Bool operands"))
                }
            }
            BinaryOp::Pipe => unreachable!(),
        }
    }

    fn check_builtin(&self, name: &str, args: &[Type]) -> EiriadResult<Type> {
        match name {
            "print" => {
                expect_arg_count(name, args, 1)?;
                Ok(Type::Unit)
            }
            "len" => {
                expect_arg_count(name, args, 1)?;
                if args[0] == Type::Str {
                    Ok(Type::Int)
                } else {
                    Err(EiriadError::new("len expects Str"))
                }
            }
            "sqrt" => {
                expect_arg_count(name, args, 1)?;
                if is_numeric(&args[0]) {
                    Ok(Type::Float)
                } else {
                    Err(EiriadError::new("sqrt expects numeric value"))
                }
            }
            "typeof" => {
                expect_arg_count(name, args, 1)?;
                Ok(Type::Str)
            }
            "fetch" | "http_get" | "http_delete" | "http_head" | "http_options" => {
                expect_arg_count(name, args, 1)?;
                if args[0] == Type::Str || args[0] == Type::Unknown {
                    Ok(Type::Result(Box::new(Type::Str), Box::new(Type::Str)))
                } else {
                    Err(EiriadError::new(format!("{} expects Str URL", name)))
                }
            }
            "http_post" | "http_put" | "http_patch" => {
                expect_arg_count(name, args, 2)?;
                let url_ok = args[0] == Type::Str || args[0] == Type::Unknown;
                let body_ok = args[1] == Type::Str || args[1] == Type::Unknown;
                if url_ok && body_ok {
                    Ok(Type::Result(Box::new(Type::Str), Box::new(Type::Str)))
                } else {
                    Err(EiriadError::new(format!(
                        "{} expects (Str URL, Str body)",
                        name
                    )))
                }
            }
            "Some" => {
                expect_arg_count(name, args, 1)?;
                Ok(Type::Option(Box::new(args[0].clone())))
            }
            "Ok" => {
                expect_arg_count(name, args, 1)?;
                Ok(Type::Result(
                    Box::new(args[0].clone()),
                    Box::new(Type::Unknown),
                ))
            }
            "Err" => {
                expect_arg_count(name, args, 1)?;
                Ok(Type::Result(
                    Box::new(Type::Unknown),
                    Box::new(args[0].clone()),
                ))
            }
            "unwrap_or" => {
                expect_arg_count(name, args, 2)?;
                match &args[0] {
                    Type::Option(inner) => {
                        if compatible(inner, &args[1]) {
                            Ok((**inner).clone())
                        } else {
                            Err(EiriadError::new(
                                "unwrap_or default type must match Option<T>",
                            ))
                        }
                    }
                    Type::Result(ok, _) => {
                        if compatible(ok, &args[1]) {
                            Ok((**ok).clone())
                        } else {
                            Err(EiriadError::new(
                                "unwrap_or default type must match Result<T, E>",
                            ))
                        }
                    }
                    _ => Err(EiriadError::new(
                        "unwrap_or expects Option<T> or Result<T, E>",
                    )),
                }
            }
            "is_some" | "is_none" => {
                expect_arg_count(name, args, 1)?;
                match &args[0] {
                    Type::Option(_) => Ok(Type::Bool),
                    _ => Err(EiriadError::new(format!("{} expects Option<T>", name))),
                }
            }
            "is_ok" | "is_err" => {
                expect_arg_count(name, args, 1)?;
                match &args[0] {
                    Type::Result(_, _) => Ok(Type::Bool),
                    _ => Err(EiriadError::new(format!("{} expects Result<T, E>", name))),
                }
            }
            _ => Err(EiriadError::new(format!("Unknown function '{}'", name))),
        }
    }
}

fn expect_arg_count(name: &str, args: &[Type], n: usize) -> EiriadResult<()> {
    if args.len() == n {
        Ok(())
    } else {
        Err(EiriadError::new(format!(
            "{} expects {} argument(s), got {}",
            name,
            n,
            args.len()
        )))
    }
}

fn is_numeric(ty: &Type) -> bool {
    ty == &Type::Int || ty == &Type::Float
}

fn comparable(a: &Type, b: &Type) -> bool {
    a == b || (is_numeric(a) && is_numeric(b))
}

fn ordered(a: &Type, b: &Type) -> bool {
    (is_numeric(a) && is_numeric(b)) || (a == &Type::Str && b == &Type::Str)
}

fn compatible(expected: &Type, found: &Type) -> bool {
    if expected == found {
        return true;
    }

    match (expected, found) {
        (Type::Unknown, _) | (_, Type::Unknown) => true,
        (Type::Fn(a), Type::Fn(b)) => a == b,
        (Type::Option(a), Type::Option(b)) => compatible(a, b),
        (Type::Result(a1, a2), Type::Result(b1, b2)) => compatible(a1, b1) && compatible(a2, b2),
        _ => false,
    }
}

fn type_name(ty: &Type) -> &'static str {
    match ty {
        Type::Int => "Int",
        Type::Float => "Float",
        Type::Bool => "Bool",
        Type::Str => "Str",
        Type::Unit => "()",
        Type::Fn(_) => "fn(_)",
        Type::Option(_) => "Option<_>",
        Type::Result(_, _) => "Result<_, _>",
        Type::Unknown => "_",
    }
}
