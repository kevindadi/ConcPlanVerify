use cir::ast::{Statement, Transfer};
use cvn::model::{BoolExpr, TransitionKind};

use super::context::{ResKind, TranslateContext, cp_id, reacquire_cp_id, rp_id, tid, wp_id};
use crate::error::TranslateError;

/// Translate `res_op(cv, wait, mtx)`.
///
/// Generates:
///   - Wait place `wp(cv, fn, sid)`
///   - `t_cv_wait`: releases the mutex, moves control to wait place
///   - Reacquire control place `cp(fn, sid_reacquire)`
///   - `t_cv_reacquire`: re-acquires the mutex, moves control to resume point
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

    // Validate the mutex exists and is a Mutex.
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
    let reacquire_cp = reacquire_cp_id(fn_name, &stmt.sid);

    // 1. Create the wait place.
    ctx.add_wait_place(cv_name, fn_name, &stmt.sid);

    // 2. Create the reacquire intermediate control place.
    ctx.ensure_control_place(fn_name, &format!("{}_reacquire", &stmt.sid));

    // 3. Ensure resume target exists.
    ctx.ensure_control_place(fn_name, &resume_sid);

    // 4. t_cv_wait: cp(fn, sid) → wp(cv, fn, sid) + rp(mtx)
    let wait_tid = tid(fn_name, &stmt.sid, "cv_wait");
    ctx.add_transition(&wait_tid, TransitionKind::CondvarWait, &[&stmt.sid]);
    ctx.add_input_arc(input_cp, &wait_tid, 1, BoolExpr::True);
    ctx.add_output_arc(&wait_tid, &wp, 1, None);
    ctx.add_output_arc(&wait_tid, &rp_id(&mutex_name), 1, None);

    // 5. t_cv_reacquire: cp(fn, sid_reacquire) + rp(mtx) → cp(fn, resume_sid)
    let reacquire_tid = tid(fn_name, &stmt.sid, "cv_reacquire");
    ctx.add_transition(&reacquire_tid, TransitionKind::CondvarReacquire, &[&stmt.sid]);
    ctx.add_input_arc(&reacquire_cp, &reacquire_tid, 1, BoolExpr::True);
    ctx.add_input_arc(&rp_id(&mutex_name), &reacquire_tid, 1, BoolExpr::True);
    ctx.add_output_arc(&reacquire_tid, &cp_id(fn_name, &resume_sid), 1, None);
}

/// Translate `res_op(cv, notify)`.
///
/// For each wait-site of this condvar, generates a transition:
///   `t_cv_notify_k`: cp(notifier, sid) + wp(cv, waiter, wait_sid) →
///                     cp(notifier, next) + cp(waiter, wait_sid_reacquire)
///
/// The transitions are in conflict (share the notifier's control place),
/// modelling nondeterministic choice of which waiter to wake.
pub(crate) fn translate_notify(
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

    for (i, ws) in wait_sites.iter().enumerate() {
        let suffix = if wait_sites.len() == 1 {
            "cv_notify".to_string()
        } else {
            format!("cv_notify_{i}")
        };
        let notify_tid = tid(fn_name, &stmt.sid, &suffix);
        let wp = wp_id(cv_name, &ws.fn_name, &ws.sid);
        let reacquire_cp = reacquire_cp_id(&ws.fn_name, &ws.sid);

        ctx.add_transition(
            &notify_tid,
            TransitionKind::CondvarNotify {
                target_wait_place: wp.clone(),
            },
            &[&stmt.sid],
        );
        ctx.add_input_arc(input_cp, &notify_tid, 1, BoolExpr::True);
        ctx.add_input_arc(&wp, &notify_tid, 1, BoolExpr::True);
        ctx.add_output_arc(&notify_tid, &target_cp, 1, None);
        ctx.add_output_arc(&notify_tid, &reacquire_cp, 1, None);
    }
}

/// Translate `res_op(cv, notify_all)`.
///
/// Uses chain-based expansion: the notifier sequentially tries to wake each
/// wait-site. At each step, two transitions branch on whether the wait-place
/// has a token (wake it) or not (skip). This produces 2*K transitions for K
/// wait-sites, which is manageable.
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

    let final_cp = match &stmt.transfer {
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

    let k = wait_sites.len();

    // Chain: input_cp → [try_wake_0] → chain_cp_1 → [try_wake_1] → ... → final_cp
    // At each step i:
    //   t_wake_i:  chain_cp_i + wp_i → chain_cp_{i+1} + reacquire_cp_i
    //   t_skip_i:  chain_cp_i → chain_cp_{i+1}   (when wp_i is empty — modelled
    //              as conflict with wake; in an over-approximation, both are enabled)
    //
    // Since we can't test for zero tokens with guards alone, we generate both
    // transitions. The state space exploration will naturally handle the
    // nondeterminism: if wp_i has a token, both are enabled; if not, only skip
    // is enabled.

    let mut current_cp = input_cp.to_string();

    for (i, ws) in wait_sites.iter().enumerate() {
        let is_last = i == k - 1;
        let next_cp = if is_last {
            final_cp.clone()
        } else {
            let chain_sid = format!("{}_na_chain_{}", &stmt.sid, i + 1);
            ctx.ensure_control_place(fn_name, &chain_sid);
            cp_id(fn_name, &chain_sid)
        };

        let wp = wp_id(cv_name, &ws.fn_name, &ws.sid);
        let reacquire_cp = reacquire_cp_id(&ws.fn_name, &ws.sid);

        // Wake transition.
        let wake_tid = tid(fn_name, &stmt.sid, &format!("na_wake_{i}"));
        ctx.add_transition(
            &wake_tid,
            TransitionKind::CondvarNotify {
                target_wait_place: wp.clone(),
            },
            &[&stmt.sid],
        );
        ctx.add_input_arc(&current_cp, &wake_tid, 1, BoolExpr::True);
        ctx.add_input_arc(&wp, &wake_tid, 1, BoolExpr::True);
        ctx.add_output_arc(&wake_tid, &next_cp, 1, None);
        ctx.add_output_arc(&wake_tid, &reacquire_cp, 1, None);

        // Skip transition (wp empty — no token to consume).
        let skip_tid = tid(fn_name, &stmt.sid, &format!("na_skip_{i}"));
        ctx.add_transition(&skip_tid, TransitionKind::CondvarNotifyAll, &[&stmt.sid]);
        ctx.add_input_arc(&current_cp, &skip_tid, 1, BoolExpr::True);
        ctx.add_output_arc(&skip_tid, &next_cp, 1, None);

        current_cp = next_cp;
    }
}
