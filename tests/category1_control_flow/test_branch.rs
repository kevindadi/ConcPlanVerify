use crate::common;
use unipn::{BoolExpr, CmpOp, Expr, Val};

#[test]
fn branch_creates_two_transitions() {
    let net = common::translate_fixture("branch.json");
    assert!(common::has_transition(&net, "main_s5_branch_true"));
    assert!(common::has_transition(&net, "main_s5_branch_false"));
}

#[test]
fn branch_shares_input_place() {
    let net = common::translate_fixture("branch.json");

    let in_t = common::input_arcs(&net, "main_s5_branch_true");
    let in_f = common::input_arcs(&net, "main_s5_branch_false");
    assert_eq!(in_t[0].0, "main.s5");
    assert_eq!(in_f[0].0, "main.s5");
}

#[test]
fn branch_targets_different_places() {
    let net = common::translate_fixture("branch.json");

    let out_t = common::output_arcs(&net, "main_s5_branch_true");
    let out_f = common::output_arcs(&net, "main_s5_branch_false");
    assert_eq!(out_t[0].0, "main.s6");
    assert_eq!(out_f[0].0, "main.s7");
}

#[test]
fn branch_guards_are_complementary() {
    let net = common::translate_fixture("branch.json");

    let g_t = common::input_guard_by_name(&net, "main_s5_branch_true", "main.s5")
        .expect("true-branch guard");
    let g_f = common::input_guard_by_name(&net, "main_s5_branch_false", "main.s5")
        .expect("false-branch guard");

    let expected_guard = BoolExpr::Cmp {
        op: CmpOp::Gt,
        lhs: Box::new(Expr::Ref("count".into())),
        rhs: Box::new(Expr::Lit(Val::int(0))),
    };

    assert_eq!(g_t, expected_guard);
    assert_eq!(g_f, BoolExpr::Not(Box::new(expected_guard)));
}
