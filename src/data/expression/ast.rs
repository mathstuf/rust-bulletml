// Distributed under the OSI-approved BSD 2-Clause License.
// See accompanying LICENSE file for details.

use crate::data::expression::Value;

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
    pub fn eval(self, v: Value) -> Value {
        match self {
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
    pub fn eval(self, l: Value, r: Value) -> Value {
        match self {
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
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        }
    }

    pub fn unary(op: UnaryOp, expr: Expr) -> Self {
        Expr::Unary {
            op,
            expr: Box::new(expr),
        }
    }

    fn constant_value(&self) -> Option<Value> {
        if let Expr::Float(v) = *self {
            Some(v)
        } else {
            None
        }
    }

    pub fn constant_fold(self) -> Self {
        match self {
            Expr::Unary {
                op: o,
                expr: e,
            } => {
                let ne = e.constant_fold();
                if let Some(v) = ne.constant_value() {
                    Expr::Float(o.eval(v))
                } else {
                    Self::unary(o, ne)
                }
            },
            Expr::Binary {
                op: o,
                lhs: l,
                rhs: r,
            } => {
                let nl = l.constant_fold();
                let nr = r.constant_fold();
                if let (Some(l), Some(r)) = (nl.constant_value(), nr.constant_value()) {
                    Expr::Float(o.eval(l, r))
                } else {
                    Self::binary(o, nl, nr)
                }
            },
            e => e,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::data::expression::ast::Expr;
    use crate::data::expression::grammar;
    use crate::data::expression::Value;

    fn parse(expr: &str) -> Expr {
        grammar::expression(expr).unwrap()
    }

    fn check_literal(actual: Expr, expected: Value) {
        check_literal_ref(&actual, expected);
    }

    fn check_literal_ref(actual: &Expr, expected: Value) {
        if let Expr::Float(actual) = *actual {
            assert_eq!(actual, expected);
        } else {
            panic!("did not parse a float: {:?}", actual);
        }
    }

    #[test]
    fn test_constant_folding_unary() {
        let expr = parse("-4").constant_fold();
        check_literal(expr, -4.);
    }

    #[test]
    fn test_constant_folding_binops() {
        let expr = parse("4+2").constant_fold();
        check_literal(expr, 6.);

        let expr = parse("4-2").constant_fold();
        check_literal(expr, 2.);

        let expr = parse("4*2").constant_fold();
        check_literal(expr, 8.);

        let expr = parse("4/2").constant_fold();
        check_literal(expr, 2.);

        let expr = parse("4%2").constant_fold();
        check_literal(expr, 0.);
    }

    #[test]
    fn test_constant_folding_parens() {
        let expr = parse("4+(2+1)").constant_fold();
        check_literal(expr, 7.);

        let expr = parse("4-(2+1)").constant_fold();
        check_literal(expr, 1.);

        let expr = parse("4*(2+1)").constant_fold();
        check_literal(expr, 12.);

        let expr = parse("4/(2+1)").constant_fold();
        check_literal(expr, 4. / 3.);

        let expr = parse("4%(2+1)").constant_fold();
        check_literal(expr, 1.);
    }

    fn eval(expr: &str) -> Value {
        parse(expr).constant_fold().constant_value().unwrap()
    }

    #[test]
    fn test_order_of_operations() {
        assert_eq!(eval("1+2*2"), 5.);
        assert_eq!(eval("2*2+1"), 5.);
    }

    #[test]
    fn test_parentheses() {
        assert_eq!(eval("(1+1)"), 2.);
        assert_eq!(eval("2*(2+1)"), 6.);
        assert_eq!(eval("(2+1)*2"), 6.);
        assert_eq!(eval("(2+1)*(2+3)"), 15.);
        assert_eq!(eval("(2*1)+(2*3)"), 8.);
        assert_eq!(eval("(4*(1+2))*2"), 24.);
        assert_eq!(eval("(4+(1+2))+2"), 9.);
        assert_eq!(eval("2*(2+1*2)"), 8.);
        assert_eq!(eval("2*(2-1*2)"), 0.);
        assert_eq!(eval("-(2)"), -2.);
        assert_eq!(eval("-(-1)"), 1.);
        assert_eq!(eval("(2*2)*(1+2)-4"), 8.);
        assert_eq!(eval("(2*2)-(1+2)*4"), -8.);
        assert_eq!(eval("2*(1-2*4)"), -14.);
    }

    #[test]
    fn test_compound() {
        assert_eq!(eval("1*-1"), -1.);
        assert_eq!(eval("(-1)"), -1.);
    }
}
