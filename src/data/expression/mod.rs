// Distributed under the OSI-approved BSD 2-Clause License.
// See accompanying LICENSE file for details.

use crates::failure::{Fallible, ResultExt};

mod ast;
mod grammar;

use self::ast::{Expr, ExprVar};

#[derive(Debug, Fail)]
pub enum ExpressionError {
    #[fail(display = "failed to parse expression")]
    ParseFailure,
    #[fail(display = "undefined variable `{}`", _0)]
    UndefinedVariable(String),
}

/// The value of an expression.
pub type Value = f32;

/// The context in which to execute an expression.
///
/// This provides values for variables referenced in expressions.
pub trait ExpressionContext {
    /// Get the value of a variable.
    fn get(&self, name: &str) -> Option<Value>;
    /// Get a random value.
    fn rand(&self) -> Value;
    /// Get the difficulty of the entity using the expression.
    fn rank(&self) -> Value;
}

#[derive(Debug, Clone)]
/// An expression which may be evaluated to compute a value.
pub struct Expression {
    expr: Expr,
}

impl Expression {
    /// Parse an expression from a string.
    pub fn parse<E>(expr: E) -> Fallible<Self>
    where
        E: AsRef<str>,
    {
        Ok(grammar::expression(expr.as_ref())
            .map(|expr| {
                Expression {
                    expr: expr.constant_fold(),
                }
            })
            .context(ExpressionError::ParseFailure)?)
    }

    /// Evaluate the expression with a given context.
    pub fn eval(&self, ctx: &ExpressionContext) -> Fallible<Value> {
        Self::eval_expr(&self.expr, ctx)
    }

    fn eval_expr(expr: &Expr, ctx: &ExpressionContext) -> Fallible<Value> {
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
                        ctx.get(&n)
                            .ok_or_else(|| ExpressionError::UndefinedVariable(n.clone()).into())
                    },
                }
            },
        }
    }
}
