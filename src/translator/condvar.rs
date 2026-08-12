use concir::ast::{Statement, Transfer};
use unipn::{BoolExpr, CmpOp, Expr, TransitionKind, Val, VarUpdate};

use super::context::{
    ResKind, TranslateContext, cp_id, na_var_name, nw_var_name, ra_id, rp_id, tid, wp_id,
};
use crate::error::TranslateError;

/// Translate `res_op(cv, wait, mtx)`.
///
/// Generates 4 transitions:
///   1. t_enter  [CondvarWaitEnter]:       cp(f,sid) → wp(sid) + rp(mtx)
///   2. t_wake1  [CondvarWakeByNotify]:    wp(sid) + rp(cv) → ra(sid)
///   3. t_wakeA  [CondvarWakeByNotifyAll]: wp(sid) → ra(sid)  [guard: na_sid == true]
///   4. t_reacq  [CondvarReacquire]:       ra(sid) + rp(mtx) → cp(f,sid')
pub(crate) fn translate_wait(
    ctx: &mut TranslateContext,
    fn_name: &str,
    stmt: &Statement,
    cv_name: &str,
    args: &[String],
    input_cp: &str,
) {
    let mutex_name = match args.first() {
        Some(m) => m.clone(),
        None => {
            ctx.push_error(TranslateError::CondvarLockNotFound(cv_name.to_string()));
            return;
        }
    };

    match ctx.resource_map.get(&mutex_name) {
        None => {
            ctx.push_error(TranslateError::CondvarLockNotFound(mutex_name.clone()));
            return;
        }
        Some(ResKind::Mutex) => {}
        Some(_) => {
            ctx.push_error(TranslateError::CondvarLockNotMutex(mutex_name.clone()));
            return;
        }
    }

    let resume_sid = match &stmt.transfer {
        Transfer::Next(s) => s.clone(),
        _ => {
            ctx.push_error(TranslateError::InvalidTarget {
                sid: stmt.sid.clone(),
                fn_name: fn_name.to_string(),
            });
            return;
        }
    };

    let wp = wp_id(cv_name, fn_name, &stmt.sid);
    let ra = ra_id(fn_name, &stmt.sid);
    let nw_var = nw_var_name(cv_name);
    let na_var = na_var_name(fn_name, &stmt.sid);

    // Create places.
    ctx.add_wait_place(cv_name, fn_name, &stmt.sid);
    ctx.ensure_reacquire_place(fn_name, &stmt.sid);
    ctx.ensure_control_place(fn_name, &resume_sid);

    // ── 1. t_enter: cp(f,sid) → wp(sid) + rp(mtx)
    //    update: nw_cv += 1, na_sid ← false
    let enter_tid = tid(fn_name, &stmt.sid, "cv_enter");
    ctx.add_transition(&enter_tid, TransitionKind::CondvarWaitEnter, &[&stmt.sid]);
    ctx.add_input_arc(input_cp, &enter_tid, 1, BoolExpr::True);
    ctx.add_output_arc(&enter_tid, &wp, 1, None);
    {
        let mut update = VarUpdate::new();
        update.insert(
            nw_var.clone(),
            Expr::BinOp {
                op: unipn::Op::Add,
                lhs: Box::new(Expr::Ref(nw_var.clone())),
                rhs: Box::new(Expr::Lit(Val::int(1))),
            },
        );
        update.insert(na_var.clone(), Expr::Lit(Val::bool(false)));
        ctx.add_output_arc(&enter_tid, &rp_id(&mutex_name), 1, Some(update));
    }

    // Wake / reacquire form one disjunctive family: only one wake path fires
    // per wait, so a never-fired sibling must not be reported as DeadTransition.
    let wait_wake_family = format!("{fn_name}_{}:wait_wake", stmt.sid);

    // ── 2. t_wake1: wp(sid) + rp(cv) → ra(sid)
    //    update: nw_cv -= 1
    let wake1_tid = tid(fn_name, &stmt.sid, "cv_wake1");
    ctx.add_transition(
        &wake1_tid,
        TransitionKind::CondvarWakeByNotify,
        &[&stmt.sid],
    );
    ctx.set_disjunctive_family(&wake1_tid, &wait_wake_family);
    ctx.add_input_arc(&wp, &wake1_tid, 1, BoolExpr::True);
    ctx.add_input_arc(&rp_id(cv_name), &wake1_tid, 1, BoolExpr::True);
    {
        let mut update = VarUpdate::new();
        update.insert(
            nw_var.clone(),
            Expr::BinOp {
                op: unipn::Op::Sub,
                lhs: Box::new(Expr::Ref(nw_var.clone())),
                rhs: Box::new(Expr::Lit(Val::int(1))),
            },
        );
        ctx.add_output_arc(&wake1_tid, &ra, 1, Some(update));
    }

    // ── 3. t_wakeA: wp(sid) → ra(sid)
    //    guard: na_sid == true
    //    update: nw_cv -= 1, na_sid ← false
    let wakea_tid = tid(fn_name, &stmt.sid, "cv_wakeA");
    ctx.add_transition(
        &wakea_tid,
        TransitionKind::CondvarWakeByNotifyAll,
        &[&stmt.sid],
    );
    ctx.set_disjunctive_family(&wakea_tid, &wait_wake_family);
    let na_guard = BoolExpr::Cmp {
        op: CmpOp::Eq,
        lhs: Box::new(Expr::Ref(na_var.clone())),
        rhs: Box::new(Expr::Lit(Val::bool(true))),
    };
    ctx.add_input_arc(&wp, &wakea_tid, 1, na_guard);
    {
        let mut update = VarUpdate::new();
        update.insert(
            nw_var.clone(),
            Expr::BinOp {
                op: unipn::Op::Sub,
                lhs: Box::new(Expr::Ref(nw_var.clone())),
                rhs: Box::new(Expr::Lit(Val::int(1))),
            },
        );
        update.insert(na_var.clone(), Expr::Lit(Val::bool(false)));
        ctx.add_output_arc(&wakea_tid, &ra, 1, Some(update));
    }

    // ── 4. t_reacq: ra(sid) + rp(mtx) → cp(f,sid')
    let reacq_tid = tid(fn_name, &stmt.sid, "cv_reacquire");
    ctx.add_transition(&reacq_tid, TransitionKind::CondvarReacquire, &[&stmt.sid]);
    ctx.set_disjunctive_family(&reacq_tid, &wait_wake_family);
    ctx.add_input_arc(&ra, &reacq_tid, 1, BoolExpr::True);
    ctx.add_input_arc(&rp_id(&mutex_name), &reacq_tid, 1, BoolExpr::True);
    ctx.add_output_arc(&reacq_tid, &cp_id(fn_name, &resume_sid), 1, None);
}

/// Translate `res_op(cv, notify)`.
///
/// Generates 2 transitions:
///   1. t_notify [CondvarNotify]:     cp(f,sid) → cp(f,sid') + rp(cv)  [guard: nw_cv > 0]
///   2. t_lost   [CondvarNotifyLost]: cp(f,sid) → cp(f,sid')           [guard: nw_cv == 0]
pub(crate) fn translate_notify(
    ctx: &mut TranslateContext,
    fn_name: &str,
    stmt: &Statement,
    cv_name: &str,
    input_cp: &str,
) {
    let target_cp = match &stmt.transfer {
        Transfer::Next(s) => {
            ctx.ensure_control_place(fn_name, s);
            cp_id(fn_name, s)
        }
        Transfer::Return => {
            ctx.ensure_return_place(fn_name);
            cp_id(fn_name, "ret")
        }
        _ => {
            ctx.push_error(TranslateError::InvalidTarget {
                sid: stmt.sid.clone(),
                fn_name: fn_name.to_string(),
            });
            return;
        }
    };

    let nw_var = nw_var_name(cv_name);

    // guard: nw_cv > 0
    let nw_gt_zero = BoolExpr::Cmp {
        op: CmpOp::Gt,
        lhs: Box::new(Expr::Ref(nw_var.clone())),
        rhs: Box::new(Expr::Lit(Val::int(0))),
    };
    // guard: nw_cv == 0
    let nw_eq_zero = BoolExpr::Cmp {
        op: CmpOp::Eq,
        lhs: Box::new(Expr::Ref(nw_var.clone())),
        rhs: Box::new(Expr::Lit(Val::int(0))),
    };

    let notify_family = format!("{fn_name}_{}:notify", stmt.sid);

    // ── 1. t_notify: cp → cp(next) + rp(cv)
    let notify_tid = tid(fn_name, &stmt.sid, "cv_notify");
    ctx.add_transition(&notify_tid, TransitionKind::CondvarNotify, &[&stmt.sid]);
    ctx.set_disjunctive_family(&notify_tid, &notify_family);
    ctx.add_input_arc(input_cp, &notify_tid, 1, nw_gt_zero);
    ctx.add_output_arc(&notify_tid, &target_cp, 1, None);
    ctx.add_output_arc(&notify_tid, &rp_id(cv_name), 1, None);

    // ── 2. t_lost: cp → cp(next)
    let lost_tid = tid(fn_name, &stmt.sid, "cv_notify_lost");
    ctx.add_transition(&lost_tid, TransitionKind::CondvarNotifyLost, &[&stmt.sid]);
    ctx.set_disjunctive_family(&lost_tid, &notify_family);
    ctx.add_input_arc(input_cp, &lost_tid, 1, nw_eq_zero);
    ctx.add_output_arc(&lost_tid, &target_cp, 1, None);
}

/// Translate `res_op(cv, notify_all)`.
///
/// Generates 2 transitions:
///   1. t_notifyAll [CondvarNotifyAll]:     cp(f,sid) → cp(f,sid')  [guard: nw_cv > 0; na_w1..wk ← true]
///   2. t_allLost   [CondvarNotifyAllLost]: cp(f,sid) → cp(f,sid')  [guard: nw_cv == 0]
pub(crate) fn translate_notify_all(
    ctx: &mut TranslateContext,
    fn_name: &str,
    stmt: &Statement,
    cv_name: &str,
    input_cp: &str,
) {
    let wait_sites = match ctx.wait_sites.get(cv_name) {
        Some(sites) if !sites.is_empty() => sites.clone(),
        _ => {
            ctx.push_error(TranslateError::NoWaitSites(cv_name.to_string()));
            return;
        }
    };

    let target_cp = match &stmt.transfer {
        Transfer::Next(s) => {
            ctx.ensure_control_place(fn_name, s);
            cp_id(fn_name, s)
        }
        Transfer::Return => {
            ctx.ensure_return_place(fn_name);
            cp_id(fn_name, "ret")
        }
        _ => {
            ctx.push_error(TranslateError::InvalidTarget {
                sid: stmt.sid.clone(),
                fn_name: fn_name.to_string(),
            });
            return;
        }
    };

    let nw_var = nw_var_name(cv_name);

    let nw_gt_zero = BoolExpr::Cmp {
        op: CmpOp::Gt,
        lhs: Box::new(Expr::Ref(nw_var.clone())),
        rhs: Box::new(Expr::Lit(Val::int(0))),
    };
    let nw_eq_zero = BoolExpr::Cmp {
        op: CmpOp::Eq,
        lhs: Box::new(Expr::Ref(nw_var.clone())),
        rhs: Box::new(Expr::Lit(Val::int(0))),
    };

    let notify_all_family = format!("{fn_name}_{}:notify_all", stmt.sid);

    // ── 1. t_notifyAll: cp → cp(next), set all na flags
    let na_tid = tid(fn_name, &stmt.sid, "cv_notify_all");
    ctx.add_transition(&na_tid, TransitionKind::CondvarNotifyAll, &[&stmt.sid]);
    ctx.set_disjunctive_family(&na_tid, &notify_all_family);
    ctx.add_input_arc(input_cp, &na_tid, 1, nw_gt_zero);
    {
        let mut update = VarUpdate::new();
        for ws in &wait_sites {
            let na_var = na_var_name(&ws.fn_name, &ws.sid);
            update.insert(na_var, Expr::Lit(Val::bool(true)));
        }
        ctx.add_output_arc(&na_tid, &target_cp, 1, Some(update));
    }

    // ── 2. t_allLost: cp → cp(next)
    let lost_tid = tid(fn_name, &stmt.sid, "cv_notify_all_lost");
    ctx.add_transition(
        &lost_tid,
        TransitionKind::CondvarNotifyAllLost,
        &[&stmt.sid],
    );
    ctx.set_disjunctive_family(&lost_tid, &notify_all_family);
    ctx.add_input_arc(input_cp, &lost_tid, 1, nw_eq_zero);
    ctx.add_output_arc(&lost_tid, &target_cp, 1, None);
}
