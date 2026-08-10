#![allow(clippy::collapsible_if)]

use concir::ast::Transfer;
use unipn::{BoolExpr, TransitionKind, VarUpdate};

use super::context::{TranslateContext, cp_id, tid};
use super::expr_parser::parse_condition;
use crate::error::TranslateError;

/// Outcome of translating a ConcIR Transfer — consumed by the operation layer
/// to wire the output side of a transition.
pub(crate) enum TransferPlan {
    /// Single successor: one transition already wired.
    Next { target_cp: String },
    /// Branch: two transitions created (true / false).
    Branch {
        true_tid: String,
        true_cp: String,
        false_tid: String,
        false_cp: String,
        guard: BoolExpr,
    },
    /// Switch: multiple transitions (one per label).
    Switch { arms: Vec<SwitchArm> },
    /// Return: target is the function's return place.
    Return { target_cp: String },
}

pub(crate) struct SwitchArm {
    pub tid: String,
    pub target_cp: String,
    pub label: String,
}

/// Analyse a ConcIR Transfer and produce a TransferPlan.
///
/// This does NOT create transitions — it merely computes the plan. The caller
/// (operation layer) decides how to combine it with the `op`.
pub(crate) fn plan_transfer(
    ctx: &mut TranslateContext,
    fn_name: &str,
    sid: &str,
    transfer: &Transfer,
) -> TransferPlan {
    match transfer {
        Transfer::Next(target_sid) => {
            ctx.ensure_control_place(fn_name, target_sid);
            TransferPlan::Next {
                target_cp: cp_id(fn_name, target_sid),
            }
        }
        Transfer::Branch {
            cond,
            true_target,
            false_target,
        } => {
            ctx.ensure_control_place(fn_name, true_target);
            ctx.ensure_control_place(fn_name, false_target);

            let guard =
                match parse_condition(cond, &ctx.all_enum_variants, ctx.aliases_for(fn_name)) {
                    Ok(g) => g,
                    Err(_) => {
                        ctx.push_error(TranslateError::InvalidBranchCondition(cond.clone()));
                        BoolExpr::True
                    }
                };

            TransferPlan::Branch {
                true_tid: tid(fn_name, sid, "branch_true"),
                true_cp: cp_id(fn_name, true_target),
                false_tid: tid(fn_name, sid, "branch_false"),
                false_cp: cp_id(fn_name, false_target),
                guard,
            }
        }
        Transfer::Switch { var: _, cases } => {
            let arms = cases
                .iter()
                .map(|(label, target_sid)| {
                    ctx.ensure_control_place(fn_name, target_sid);
                    SwitchArm {
                        tid: tid(fn_name, sid, &format!("switch_{label}")),
                        target_cp: cp_id(fn_name, target_sid),
                        label: label.clone(),
                    }
                })
                .collect();
            TransferPlan::Switch { arms }
        }
        Transfer::Return => {
            ctx.ensure_return_place(fn_name);
            TransferPlan::Return {
                target_cp: cp_id(fn_name, "ret"),
            }
        }
    }
}

/// Emit a single transition with one control-flow input arc and one
/// control-flow output arc. Used for simple sequential operations.
#[allow(clippy::too_many_arguments)]
pub(crate) fn emit_simple_transition(
    ctx: &mut TranslateContext,
    transition_id: &str,
    kind: TransitionKind,
    anchor_sids: &[&str],
    input_cp: &str,
    output_cp: &str,
    guard: BoolExpr,
    update: Option<VarUpdate>,
) {
    ctx.add_transition(transition_id, kind, anchor_sids);
    ctx.add_input_arc(input_cp, transition_id, 1, guard);
    ctx.add_output_arc(transition_id, output_cp, 1, update);
}

/// Emit branch-pair transitions sharing the same control-flow input.
#[allow(clippy::too_many_arguments)]
pub(crate) fn emit_branch_transitions(
    ctx: &mut TranslateContext,
    anchor_sids: &[&str],
    input_cp: &str,
    true_tid: &str,
    true_cp: &str,
    false_tid: &str,
    false_cp: &str,
    guard: BoolExpr,
) {
    let neg_guard = BoolExpr::Not(Box::new(guard.clone()));

    ctx.add_transition(true_tid, TransitionKind::BranchTrue, anchor_sids);
    ctx.add_input_arc(input_cp, true_tid, 1, guard);
    ctx.add_output_arc(true_tid, true_cp, 1, None);

    ctx.add_transition(false_tid, TransitionKind::BranchFalse, anchor_sids);
    ctx.add_input_arc(input_cp, false_tid, 1, neg_guard);
    ctx.add_output_arc(false_tid, false_cp, 1, None);
}

/// Emit switch transitions sharing the same control-flow input, with
/// per-label guards.
#[allow(clippy::too_many_arguments)]
pub(crate) fn emit_switch_transitions(
    ctx: &mut TranslateContext,
    anchor_sids: &[&str],
    input_cp: &str,
    switch_var: &str,
    arms: &[SwitchArm],
) {
    for arm in arms {
        let guard = unipn::BoolExpr::Cmp {
            op: unipn::CmpOp::Eq,
            lhs: Box::new(unipn::Expr::Ref(switch_var.to_string())),
            rhs: Box::new(unipn::Expr::Lit(unipn::Val::enum_val(&arm.label))),
        };
        ctx.add_transition(
            &arm.tid,
            TransitionKind::Switch {
                label: arm.label.clone(),
            },
            anchor_sids,
        );
        ctx.add_input_arc(input_cp, &arm.tid, 1, guard);
        ctx.add_output_arc(&arm.tid, &arm.target_cp, 1, None);
    }
}
