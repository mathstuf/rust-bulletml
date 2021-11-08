// Distributed under the OSI-approved BSD 2-Clause License.
// See accompanying LICENSE file for details.

use serde::de::{Deserializer, Error, Unexpected};
use serde::Deserialize;
use thiserror::Error;

mod ast;
mod grammar;

use self::ast::{Expr, ExprVar};

/// An error when evaluating an expression.
#[derive(Debug, Error)]
pub enum ExpressionError {
    /// Failed to parse an expression.
    #[error("failed to parse expression")]
    ParseFailure {
        /// The parser error.
        #[from]
        source: peg::error::ParseError<peg::str::LineCol>,
    },
    /// Reference to an undefined variable.
    #[error("undefined variable `{}`", name)]
    UndefinedVariable {
        /// The name of the variable.
        name: String,
    },
    /// Reference to a missing parameter.
    #[error("missing parameter `{}`", idx)]
    MissingParameter {
        /// The index
        idx: usize,
    },
}

impl ExpressionError {
    fn undefined_variable<N>(name: N) -> Self
    where
        N: Into<String>,
    {
        Self::UndefinedVariable {
            name: name.into(),
        }
    }

    fn missing_parameter(idx: usize) -> Self {
        Self::MissingParameter {
            idx,
        }
    }
}

/// The value of an expression.
pub type Value = f32;

/// The context in which to execute an expression.
///
/// This provides values for variables referenced in expressions.
pub trait ExpressionContext {
    /// Get the value of a variable.
    fn get(&self, name: &str) -> Option<Value>;
    /// Get a parameter.
    fn get_param(&self, idx: usize) -> Option<Value>;
    /// Get a random value.
    fn rand(&self) -> Value;
    /// Get the difficulty of the entity using the expression.
    fn rank(&self) -> Value;
}

/// An expression which may be evaluated to compute a value.
#[derive(Debug, Clone)]
pub struct Expression {
    expr: Expr,
}

impl Expression {
    /// Parse an expression from a string.
    pub fn parse<E>(expr: E) -> Result<Self, ExpressionError>
    where
        E: AsRef<str>,
    {
        Ok(grammar::expression(expr.as_ref()).map(|expr| {
            Expression {
                expr: expr.constant_fold(),
            }
        })?)
    }

    /// Evaluate the expression with a given context.
    pub fn eval(&self, ctx: &dyn ExpressionContext) -> Result<Value, ExpressionError> {
        Self::eval_expr(&self.expr, ctx)
    }

    fn eval_expr(expr: &Expr, ctx: &dyn ExpressionContext) -> Result<Value, ExpressionError> {
        match *expr {
            Expr::Unary {
                op: ref o,
                expr: ref e,
            } => Self::eval_expr(e.as_ref(), ctx).map(|r| o.eval(r)),
            Expr::Binary {
                op: ref o,
                lhs: ref l,
                rhs: ref r,
            } => {
                Self::eval_expr(l.as_ref(), ctx)
                    .and_then(|lr| Self::eval_expr(r.as_ref(), ctx).map(|rr| o.eval(lr, rr)))
            },
            Expr::Float(f) => Ok(f),
            Expr::Var(ref v) => {
                match *v {
                    ExprVar::Rank => Ok(ctx.rank()),
                    ExprVar::Rand => Ok(ctx.rand()),
                    ExprVar::Named(ref n) => {
                        ctx.get(n)
                            .ok_or_else(|| ExpressionError::undefined_variable(n))
                    },
                    ExprVar::Param(n) => {
                        ctx.get_param(n)
                            .ok_or_else(|| ExpressionError::missing_parameter(n))
                    },
                }
            },
        }
    }
}

impl<'de> Deserialize<'de> for Expression {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let expr = String::deserialize(deserializer)?;

        Self::parse(&expr)
            .map_err(|_| D::Error::invalid_value(Unexpected::Str(&expr), &"a BulletML expression"))
    }
}
