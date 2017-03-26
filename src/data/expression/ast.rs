// Distributed under the OSI-approved BSD 2-Clause License.
// See accompanying file LICENSE for details.

use data::expression::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExprVar {
    Rank,
    Rand,
    Named(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Negate,
}

impl UnaryOp {
    pub fn eval(&self, v: Value) -> Value {
        match *self {
            UnaryOp::Negate => -v,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

impl BinaryOp {
    pub fn eval(&self, l: Value, r: Value) -> Value {
        match *self {
            BinaryOp::Add => l + r,
            BinaryOp::Sub => l - r,
            BinaryOp::Mul => l * r,
            BinaryOp::Div => l / r,
            BinaryOp::Mod => l % r,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Expr {
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Binary {
        op: BinaryOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Float(Value),
    Var(ExprVar),
}

impl Expr {
    pub fn binary(op: BinaryOp, lhs: Expr, rhs: Expr) -> Self {
        Expr::Binary {
            op: op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        }
    }

    pub fn unary(op: UnaryOp, expr: Expr) -> Self {
        Expr::Unary {
            op: op,
            expr: Box::new(expr),
        }
    }
}
