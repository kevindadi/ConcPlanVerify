use crate::common;
use cvn::model::{BoolExpr, CmpOp, Expr, PlaceId, TransitionId, Val};

#[test]
fn branch_creates_two_transitions() {
    let net = common::translate_fixture("branch.json");

    let t_true = net.transition(&TransitionId::new("main_s5_branch_true"));
    let t_false = net.transition(&TransitionId::new("main_s5_branch_false"));
    assert!(t_true.is_some());
    assert!(t_false.is_some());
}

#[test]
fn branch_shares_input_place() {
    let net = common::translate_fixture("branch.json");

    let in_t = net.input_arcs(&TransitionId::new("main_s5_branch_true"));
    let in_f = net.input_arcs(&TransitionId::new("main_s5_branch_false"));
    assert_eq!(in_t[0].place, PlaceId::new("cp_main_s5"));
    assert_eq!(in_f[0].place, PlaceId::new("cp_main_s5"));
}

#[test]
fn branch_targets_different_places() {
    let net = common::translate_fixture("branch.json");

    let out_t = net.output_arcs(&TransitionId::new("main_s5_branch_true"));
    let out_f = net.output_arcs(&TransitionId::new("main_s5_branch_false"));
    assert_eq!(out_t[0].place, PlaceId::new("cp_main_s6"));
    assert_eq!(out_f[0].place, PlaceId::new("cp_main_s7"));
}

#[test]
fn branch_guards_are_complementary() {
    let net = common::translate_fixture("branch.json");

    let in_t = net.input_arcs(&TransitionId::new("main_s5_branch_true"));
    let in_f = net.input_arcs(&TransitionId::new("main_s5_branch_false"));

    let expected_guard = BoolExpr::Cmp {
        op: CmpOp::Gt,
        lhs: Box::new(Expr::Ref("count".into())),
        rhs: Box::new(Expr::Lit(Val::int(0))),
    };

    assert_eq!(in_t[0].guard, expected_guard);
    assert_eq!(in_f[0].guard, BoolExpr::Not(Box::new(expected_guard)));
}
