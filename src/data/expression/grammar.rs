// Distributed under the OSI-approved BSD 2-Clause License.
// See accompanying LICENSE file for details.

include!(concat!(env!("OUT_DIR"), "/grammar.rs"));

#[cfg(test)]
mod test {
    use data::expression::ast::{BinaryOp, Expr, ExprVar, UnaryOp};
    use data::expression::grammar;
    use data::expression::Value;

    #[test]
    fn test_parse_paren_mismatch_fail() {
        let err = grammar::expression("(").unwrap_err();

        assert_eq!(err.line, 1);
        assert_eq!(err.column, 2);
        assert_eq!(err.offset, 1);
    }

    #[test]
    fn test_parse_lonely_binop_fail() {
        let err = grammar::expression("+").unwrap_err();

        assert_eq!(err.line, 1);
        assert_eq!(err.column, 1);
        assert_eq!(err.offset, 0);
    }

    #[test]
    fn test_parse_half_binop_fail() {
        let err = grammar::expression("4+").unwrap_err();

        assert_eq!(err.line, 1);
        assert_eq!(err.column, 3);
        assert_eq!(err.offset, 2);
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
    fn test_parse_literal() {
        let res = grammar::expression("4").unwrap();

        check_literal(res, 4.);
    }

    #[test]
    fn test_parse_literal_float() {
        let res = grammar::expression("4.").unwrap();

        check_literal(res, 4.);
    }

    #[test]
    fn test_parse_literal_float_implicit_zero() {
        let res = grammar::expression(".5").unwrap();

        check_literal(res, 0.5);
    }

    #[test]
    fn test_parse_literal_float_decimals() {
        let res = grammar::expression("4.5").unwrap();

        check_literal(res, 4.5);
    }

    fn check_binop(actual: Expr, op: BinaryOp, lhs: Value, rhs: Value) {
        if let Expr::Binary {
            op: aop,
            lhs: alhs,
            rhs: arhs,
        } = actual
        {
            assert_eq!(aop, op);
            check_literal_ref(alhs.as_ref(), lhs);
            check_literal_ref(arhs.as_ref(), rhs);
        } else {
            panic!("did not parse a binary operation: {:?}", actual);
        }
    }

    #[test]
    fn test_parse_binary_ops() {
        let res = grammar::expression("4+2").unwrap();
        check_binop(res, BinaryOp::Add, 4., 2.);

        let res = grammar::expression("4-2").unwrap();
        check_binop(res, BinaryOp::Sub, 4., 2.);

        let res = grammar::expression("4*2").unwrap();
        check_binop(res, BinaryOp::Mul, 4., 2.);

        let res = grammar::expression("4/2").unwrap();
        check_binop(res, BinaryOp::Div, 4., 2.);

        let res = grammar::expression("4%2").unwrap();
        check_binop(res, BinaryOp::Mod, 4., 2.);
    }

    fn check_unaryop(actual: Expr, op: UnaryOp, expected: Value) {
        if let Expr::Unary {
            op: aop,
            expr: aexpr,
        } = actual
        {
            assert_eq!(aop, op);
            check_literal_ref(aexpr.as_ref(), expected);
        } else {
            panic!("did not parse an unary operation: {:?}", actual);
        }
    }

    #[test]
    fn test_parse_unary_ops() {
        let res = grammar::expression("-4").unwrap();
        check_unaryop(res, UnaryOp::Negate, 4.);
    }

    fn check_variable(actual: Expr, expected: ExprVar) {
        if let Expr::Var(actual) = actual {
            assert_eq!(actual, expected);
        } else {
            panic!("did not parse a variable: {:?}", actual);
        }
    }

    #[test]
    fn test_parse_rank() {
        let res = grammar::expression("$rank").unwrap();
        check_variable(res, ExprVar::Rank);
    }

    #[test]
    fn test_parse_rand() {
        let res = grammar::expression("$rand").unwrap();
        check_variable(res, ExprVar::Rand);
    }

    #[test]
    fn test_parse_variable() {
        let res = grammar::expression("$var").unwrap();
        check_variable(res, ExprVar::Named("var".into()));
    }

    #[test]
    fn test_parse_rank_trailing() {
        let res = grammar::expression("$rankvar").unwrap();
        check_variable(res, ExprVar::Named("rankvar".into()));
    }

    #[test]
    fn test_parse_rand_trailing() {
        let res = grammar::expression("$randvar").unwrap();
        check_variable(res, ExprVar::Named("randvar".into()));
    }
}
