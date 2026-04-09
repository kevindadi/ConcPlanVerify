use cir::ast::{Function, Op, Statement, Transfer};
use cvn::model::{BoolExpr, CmpOp, Expr, TransitionKind, Val, VarUpdate};
use super::condvar;
use super::context::{LockKind, ResKind, TranslateContext, cp_id, na_var_name, rp_id, tid};
use super::control_flow::{
    TransferPlan, emit_branch_transitions, emit_simple_transition, emit_switch_transitions,
    plan_transfer,
};
use super::expr_parser::parse_expr;
use crate::error::TranslateError;

/// Phase 2: Translate all function bodies.
pub(crate) fn translate_functions(
    ctx: &mut TranslateContext,
    functions: &[Function],
) {
    // Pre-scan: collect condvar wait-sites and mark post-wait locks.
    for func in functions {
        prescan_condvar_waits(ctx, &func.name, &func.body);
    }

    // Main translation pass.
    for func in functions {
        translate_function(ctx, func);
    }
}

/// Pre-scan a function body to collect condvar wait-sites and mark the
/// subsequent lock (if any) as a post-wait lock.
fn prescan_condvar_waits(ctx: &mut TranslateContext, fn_name: &str, body: &[Statement]) {
    for (i, stmt) in body.iter().enumerate() {
        if let Op::ResOp {
            resource,
            action,
            args,
        } = &stmt.op
        {
            if action == "wait" {
                let cv_name = resource.clone();
                let mutex_name = args.first().cloned().unwrap_or_default();

                ctx.wait_sites
                    .entry(cv_name)
                    .or_default()
                    .push(super::context::WaitSite {
                        fn_name: fn_name.to_string(),
                        sid: stmt.sid.clone(),
                        mutex: mutex_name.clone(),
                    });

                // Register per-wait-site notify-all flag variable.
                let na_var = na_var_name(fn_name, &stmt.sid);
                ctx.add_variable(&na_var, Val::bool(false));

                // If the transfer target is a lock on the same mutex, mark it.
                if let Transfer::Next(resume_sid) = &stmt.transfer {
                    if let Some(next_stmt) = body.iter().find(|s| s.sid == *resume_sid) {
                        if let Op::ResOp {
                            resource: next_res,
                            action: next_act,
                            ..
                        } = &next_stmt.op
                        {
                            if next_act == "lock" && *next_res == mutex_name {
                                ctx.post_wait_locks.insert(
                                    (fn_name.to_string(), resume_sid.clone()),
                                    mutex_name.clone(),
                                );
                            }
                        }
                    }
                }

                let _ = i; // suppress unused warning
            }
        }
    }
}

fn translate_function(ctx: &mut TranslateContext, func: &Function) {
    // Ensure the first statement's control place gets an initial token
    // only if this is the entry function (handled by the orchestrator).
    // Here we just ensure places exist.
    if let Some(first) = func.body.first() {
        ctx.ensure_control_place(&func.name, &first.sid);
    }
    ctx.ensure_return_place(&func.name);

    for stmt in &func.body {
        translate_statement(ctx, &func.name, stmt);
    }
}

fn translate_statement(ctx: &mut TranslateContext, fn_name: &str, stmt: &Statement) {
    ctx.ensure_control_place(fn_name, &stmt.sid);
    let input_cp = cp_id(fn_name, &stmt.sid);

    match &stmt.op {
        Op::ResOp {
            resource,
            action,
            args,
        } => {
            translate_res_op(ctx, fn_name, stmt, resource, action, args, &input_cp);
        }
        Op::Spawn(f) | Op::SpawnAsync(f) => {
            translate_spawn(ctx, fn_name, stmt, f, &input_cp);
        }
        Op::Join(f) | Op::Await(f) => {
            translate_join(ctx, fn_name, stmt, f, &input_cp);
        }
        Op::Call(f) => {
            translate_call(ctx, fn_name, stmt, f, &input_cp);
        }
        Op::Return => {
            translate_return_op(ctx, fn_name, stmt, &input_cp);
        }
        Op::Nop => {
            translate_nop(ctx, fn_name, stmt, &input_cp);
        }
    }
}

// ── res_op dispatch ─────────────────────────────────────────────────────

fn translate_res_op(
    ctx: &mut TranslateContext,
    fn_name: &str,
    stmt: &Statement,
    resource: &str,
    action: &str,
    args: &[String],
    input_cp: &str,
) {
    let res_kind = ctx.resource_map.get(resource).cloned();

    match action {
        "lock" => translate_lock(ctx, fn_name, stmt, resource, input_cp, &res_kind),
        "drop" => translate_drop(ctx, fn_name, stmt, resource, input_cp, &res_kind),
        "read" => translate_read(ctx, fn_name, stmt, resource, input_cp, &res_kind),
        "write" => translate_write(ctx, fn_name, stmt, resource, args, input_cp),
        "send" => translate_send(ctx, fn_name, stmt, resource, args, input_cp),
        "recv" => translate_recv(ctx, fn_name, stmt, resource, input_cp),
        "acquire" => translate_lock(ctx, fn_name, stmt, resource, input_cp, &res_kind),
        "release" => translate_drop(ctx, fn_name, stmt, resource, input_cp, &res_kind),
        "load" => translate_read(ctx, fn_name, stmt, resource, input_cp, &res_kind),
        "store" => translate_store(ctx, fn_name, stmt, resource, args, input_cp),
        "cas" => translate_cas(ctx, fn_name, stmt, resource, args, input_cp),
        "wait" => condvar::translate_wait(ctx, fn_name, stmt, resource, args, input_cp),
        "notify" => condvar::translate_notify(ctx, fn_name, stmt, resource, input_cp),
        "notify_all" => condvar::translate_notify_all(ctx, fn_name, stmt, resource, input_cp),
        _ => {
            ctx.push_error(TranslateError::UnknownResourceType(format!(
                "{resource}.{action}"
            )));
        }
    }
}

// ── lock ────────────────────────────────────────────────────────────────

fn translate_lock(
    ctx: &mut TranslateContext,
    fn_name: &str,
    stmt: &Statement,
    resource: &str,
    input_cp: &str,
    res_kind: &Option<ResKind>,
) {
    let plan = plan_transfer(ctx, fn_name, &stmt.sid, &stmt.transfer);

    // Check if this is a post-wait lock (already acquired by reacquire).
    let is_post_wait = ctx
        .post_wait_locks
        .get(&(fn_name.to_string(), stmt.sid.clone()))
        .is_some();

    if is_post_wait {
        // Translate as Sequential — lock already held.
        if let TransferPlan::Next { target_cp } = plan {
            let t_id = tid(fn_name, &stmt.sid, "seq");
            emit_simple_transition(
                ctx,
                &t_id,
                TransitionKind::Sequential,
                &[&stmt.sid],
                input_cp,
                &target_cp,
                BoolExpr::True,
                None,
            );
        }
        return;
    }

    let (weight, kind, suffix) = match res_kind {
        Some(ResKind::Semaphore { .. }) => (1, TransitionKind::Acquire, "acquire"),
        Some(ResKind::RwLock) => {
            ctx.lock_tracker.insert(
                (fn_name.to_string(), resource.to_string()),
                LockKind::Write,
            );
            (ctx.rwlock_n, TransitionKind::Lock, "lock")
        }
        _ => (1, TransitionKind::Lock, "lock"),
    };

    if let TransferPlan::Next { target_cp } = plan {
        let t_id = tid(fn_name, &stmt.sid, suffix);
        ctx.add_transition(&t_id, kind, &[&stmt.sid]);
        ctx.add_input_arc(input_cp, &t_id, 1, BoolExpr::True);
        ctx.add_input_arc(&rp_id(resource), &t_id, weight, BoolExpr::True);
        ctx.add_output_arc(&t_id, &target_cp, 1, None);
    }
}

// ── read (RwLock read-lock) ─────────────────────────────────────────────

fn translate_rw_read_lock(
    ctx: &mut TranslateContext,
    fn_name: &str,
    stmt: &Statement,
    resource: &str,
    input_cp: &str,
) {
    ctx.lock_tracker.insert(
        (fn_name.to_string(), resource.to_string()),
        LockKind::Read,
    );
    let plan = plan_transfer(ctx, fn_name, &stmt.sid, &stmt.transfer);
    if let TransferPlan::Next { target_cp } = plan {
        let t_id = tid(fn_name, &stmt.sid, "read_lock");
        ctx.add_transition(&t_id, TransitionKind::ReadLock, &[&stmt.sid]);
        ctx.add_input_arc(input_cp, &t_id, 1, BoolExpr::True);
        ctx.add_input_arc(&rp_id(resource), &t_id, 1, BoolExpr::True);
        ctx.add_output_arc(&t_id, &target_cp, 1, None);
    }
}

// ── drop / release ──────────────────────────────────────────────────────

fn translate_drop(
    ctx: &mut TranslateContext,
    fn_name: &str,
    stmt: &Statement,
    resource: &str,
    input_cp: &str,
    res_kind: &Option<ResKind>,
) {
    let plan = plan_transfer(ctx, fn_name, &stmt.sid, &stmt.transfer);

    let (weight, kind, suffix) = match res_kind {
        Some(ResKind::Semaphore { .. }) => (1, TransitionKind::Release, "release"),
        Some(ResKind::RwLock) => {
            let key = (fn_name.to_string(), resource.to_string());
            match ctx.lock_tracker.get(&key) {
                Some(LockKind::Write) => (ctx.rwlock_n, TransitionKind::Unlock, "unlock"),
                Some(LockKind::Read) => (1, TransitionKind::ReadUnlock, "read_unlock"),
                None => {
                    ctx.push_error(TranslateError::AmbiguousRwLockDrop {
                        fn_name: fn_name.to_string(),
                        sid: stmt.sid.clone(),
                    });
                    (1, TransitionKind::Unlock, "unlock")
                }
            }
        }
        _ => (1, TransitionKind::Unlock, "unlock"),
    };

    if let TransferPlan::Next { target_cp } = plan {
        let t_id = tid(fn_name, &stmt.sid, suffix);
        ctx.add_transition(&t_id, kind, &[&stmt.sid]);
        ctx.add_input_arc(input_cp, &t_id, 1, BoolExpr::True);
        ctx.add_output_arc(&t_id, &target_cp, 1, None);
        ctx.add_output_arc(&t_id, &rp_id(resource), weight, None);
    }
}

// ── read (Var / Atomic load) ────────────────────────────────────────────

fn translate_read(
    ctx: &mut TranslateContext,
    fn_name: &str,
    stmt: &Statement,
    resource: &str,
    input_cp: &str,
    res_kind: &Option<ResKind>,
) {
    // RwLock "read" is a read-lock, not a variable read.
    if matches!(res_kind, Some(ResKind::RwLock)) {
        return translate_rw_read_lock(ctx, fn_name, stmt, resource, input_cp);
    }

    let plan = plan_transfer(ctx, fn_name, &stmt.sid, &stmt.transfer);

    let is_atomic = matches!(res_kind, Some(ResKind::Atomic { .. }));
    let (kind, suffix) = if is_atomic {
        (TransitionKind::AtomicLoad, "atomic_load")
    } else {
        (TransitionKind::VarRead, "var_read")
    };

    match plan {
        TransferPlan::Next { target_cp } => {
            let t_id = tid(fn_name, &stmt.sid, suffix);
            emit_simple_transition(
                ctx,
                &t_id,
                kind,
                &[&stmt.sid],
                input_cp,
                &target_cp,
                BoolExpr::True,
                None,
            );
        }
        TransferPlan::Branch {
            true_tid,
            true_cp,
            false_tid,
            false_cp,
            guard,
        } => {
            emit_branch_transitions(
                ctx,
                &[&stmt.sid],
                input_cp,
                &true_tid,
                &true_cp,
                &false_tid,
                &false_cp,
                guard,
            );
        }
        TransferPlan::Switch { arms } => {
            let switch_var = match &stmt.transfer {
                Transfer::Switch { var, .. } => var.as_str(),
                _ => resource,
            };
            emit_switch_transitions(ctx, &[&stmt.sid], input_cp, switch_var, &arms);
        }
        TransferPlan::Return { target_cp } => {
            let t_id = tid(fn_name, &stmt.sid, suffix);
            emit_simple_transition(
                ctx,
                &t_id,
                kind,
                &[&stmt.sid],
                input_cp,
                &target_cp,
                BoolExpr::True,
                None,
            );
        }
    }
}

// ── write (Var) ─────────────────────────────────────────────────────────

fn translate_write(
    ctx: &mut TranslateContext,
    fn_name: &str,
    stmt: &Statement,
    resource: &str,
    args: &[String],
    input_cp: &str,
) {
    let plan = plan_transfer(ctx, fn_name, &stmt.sid, &stmt.transfer);

    let value_expr = if let Some(val_str) = args.first() {
        parse_expr(val_str, &ctx.all_enum_variants).unwrap_or(Expr::Lit(Val::Unknown))
    } else {
        Expr::Lit(Val::Unknown)
    };

    let mut update = VarUpdate::new();
    update.insert(resource.to_string(), value_expr);

    if let TransferPlan::Next { target_cp } = plan {
        let t_id = tid(fn_name, &stmt.sid, "var_write");
        emit_simple_transition(
            ctx,
            &t_id,
            TransitionKind::VarWrite,
            &[&stmt.sid],
            input_cp,
            &target_cp,
            BoolExpr::True,
            Some(update),
        );
    }
}

// ── send ────────────────────────────────────────────────────────────────

fn translate_send(
    ctx: &mut TranslateContext,
    fn_name: &str,
    stmt: &Statement,
    resource: &str,
    _args: &[String],
    input_cp: &str,
) {
    let plan = plan_transfer(ctx, fn_name, &stmt.sid, &stmt.transfer);

    if let TransferPlan::Next { target_cp } = plan {
        let t_id = tid(fn_name, &stmt.sid, "send");
        ctx.add_transition(&t_id, TransitionKind::Send, &[&stmt.sid]);
        ctx.add_input_arc(input_cp, &t_id, 1, BoolExpr::True);
        ctx.add_output_arc(&t_id, &target_cp, 1, None);
        ctx.add_output_arc(&t_id, &rp_id(resource), 1, None);
    }
}

// ── recv ────────────────────────────────────────────────────────────────

fn translate_recv(
    ctx: &mut TranslateContext,
    fn_name: &str,
    stmt: &Statement,
    resource: &str,
    input_cp: &str,
) {
    let plan = plan_transfer(ctx, fn_name, &stmt.sid, &stmt.transfer);

    if let TransferPlan::Next { target_cp } = plan {
        let t_id = tid(fn_name, &stmt.sid, "recv");
        ctx.add_transition(&t_id, TransitionKind::Recv, &[&stmt.sid]);
        ctx.add_input_arc(input_cp, &t_id, 1, BoolExpr::True);
        ctx.add_input_arc(&rp_id(resource), &t_id, 1, BoolExpr::True);
        ctx.add_output_arc(&t_id, &target_cp, 1, None);
    }
}

// ── store (Atomic) ──────────────────────────────────────────────────────

fn translate_store(
    ctx: &mut TranslateContext,
    fn_name: &str,
    stmt: &Statement,
    resource: &str,
    args: &[String],
    input_cp: &str,
) {
    let plan = plan_transfer(ctx, fn_name, &stmt.sid, &stmt.transfer);

    let value_expr = if let Some(val_str) = args.first() {
        parse_expr(val_str, &ctx.all_enum_variants).unwrap_or(Expr::Lit(Val::Unknown))
    } else {
        Expr::Lit(Val::Unknown)
    };

    let mut update = VarUpdate::new();
    update.insert(resource.to_string(), value_expr);

    if let TransferPlan::Next { target_cp } = plan {
        let t_id = tid(fn_name, &stmt.sid, "atomic_store");
        emit_simple_transition(
            ctx,
            &t_id,
            TransitionKind::AtomicStore,
            &[&stmt.sid],
            input_cp,
            &target_cp,
            BoolExpr::True,
            Some(update),
        );
    }
}

// ── CAS (Atomic) ────────────────────────────────────────────────────────

fn translate_cas(
    ctx: &mut TranslateContext,
    fn_name: &str,
    stmt: &Statement,
    resource: &str,
    args: &[String],
    input_cp: &str,
) {
    let plan = plan_transfer(ctx, fn_name, &stmt.sid, &stmt.transfer);

    let expected = args
        .first()
        .and_then(|s| parse_expr(s, &ctx.all_enum_variants).ok())
        .unwrap_or(Expr::Lit(Val::Unknown));
    let desired = args
        .get(1)
        .and_then(|s| parse_expr(s, &ctx.all_enum_variants).ok())
        .unwrap_or(Expr::Lit(Val::Unknown));

    let success_guard = BoolExpr::Cmp {
        op: CmpOp::Eq,
        lhs: Box::new(Expr::Ref(resource.to_string())),
        rhs: Box::new(expected.clone()),
    };
    let failure_guard = BoolExpr::Not(Box::new(success_guard.clone()));

    let mut success_update = VarUpdate::new();
    success_update.insert(resource.to_string(), desired);

    match plan {
        TransferPlan::Branch {
            true_tid,
            true_cp,
            false_tid,
            false_cp,
            ..
        } => {
            // CAS + branch: success → true target, failure → false target.
            ctx.add_transition(&true_tid, TransitionKind::CasSuccess, &[&stmt.sid]);
            ctx.add_input_arc(input_cp, &true_tid, 1, success_guard.clone());
            ctx.add_output_arc(&true_tid, &true_cp, 1, Some(success_update));

            ctx.add_transition(&false_tid, TransitionKind::CasFailure, &[&stmt.sid]);
            ctx.add_input_arc(input_cp, &false_tid, 1, failure_guard);
            ctx.add_output_arc(&false_tid, &false_cp, 1, None);
        }
        TransferPlan::Next { target_cp } => {
            // CAS without branch: generate two transitions, both going to next.
            let succ_tid = tid(fn_name, &stmt.sid, "cas_succ");
            let fail_tid = tid(fn_name, &stmt.sid, "cas_fail");

            ctx.add_transition(&succ_tid, TransitionKind::CasSuccess, &[&stmt.sid]);
            ctx.add_input_arc(input_cp, &succ_tid, 1, success_guard);
            ctx.add_output_arc(&succ_tid, &target_cp, 1, Some(success_update));

            ctx.add_transition(&fail_tid, TransitionKind::CasFailure, &[&stmt.sid]);
            ctx.add_input_arc(input_cp, &fail_tid, 1, failure_guard);
            ctx.add_output_arc(&fail_tid, &target_cp, 1, None);
        }
        _ => {}
    }
}

// ── spawn ───────────────────────────────────────────────────────────────

fn translate_spawn(
    ctx: &mut TranslateContext,
    fn_name: &str,
    stmt: &Statement,
    target_fn: &str,
    input_cp: &str,
) {
    let plan = plan_transfer(ctx, fn_name, &stmt.sid, &stmt.transfer);

    if let TransferPlan::Next { target_cp } = plan {
        let t_id = tid(fn_name, &stmt.sid, "spawn");
        ctx.add_transition(&t_id, TransitionKind::Spawn, &[&stmt.sid]);
        ctx.add_input_arc(input_cp, &t_id, 1, BoolExpr::True);
        ctx.add_output_arc(&t_id, &target_cp, 1, None);

        // Also produce a token at the spawned function's first statement.
        // We need to find its first sid. The orchestrator ensures this place exists.
        let spawned_first_cp = cp_id(target_fn, "s_first");
        // We rely on the spawned function's first place already existing
        // from translate_function. Use a marker place that the orchestrator
        // will link up.
        ctx.ensure_control_place(target_fn, "s_first");
        ctx.add_output_arc(&t_id, &spawned_first_cp, 1, None);
    }
}

// ── join ────────────────────────────────────────────────────────────────

fn translate_join(
    ctx: &mut TranslateContext,
    fn_name: &str,
    stmt: &Statement,
    target_fn: &str,
    input_cp: &str,
) {
    let plan = plan_transfer(ctx, fn_name, &stmt.sid, &stmt.transfer);

    if let TransferPlan::Next { target_cp } | TransferPlan::Return { target_cp } = plan {
        let t_id = tid(fn_name, &stmt.sid, "join");
        ctx.add_transition(&t_id, TransitionKind::Join, &[&stmt.sid]);
        ctx.add_input_arc(input_cp, &t_id, 1, BoolExpr::True);
        // Also consume the spawned function's return token.
        ctx.ensure_return_place(target_fn);
        ctx.add_input_arc(&cp_id(target_fn, "ret"), &t_id, 1, BoolExpr::True);
        ctx.add_output_arc(&t_id, &target_cp, 1, None);
    }
}

// ── call ────────────────────────────────────────────────────────────────

fn translate_call(
    ctx: &mut TranslateContext,
    fn_name: &str,
    stmt: &Statement,
    target_fn: &str,
    input_cp: &str,
) {
    // If the target has a FnSummary, it is handled in Phase 3.
    // If it has a body, it is already translated (all functions are
    // processed in Phase 2). For now, generate a simple Call transition
    // with unknown writes from summary if available, or a plain sequential.
    let summary = ctx.fn_summary_map.get(target_fn).cloned();

    let plan = plan_transfer(ctx, fn_name, &stmt.sid, &stmt.transfer);

    if let TransferPlan::Next { target_cp } = plan {
        let t_id = tid(fn_name, &stmt.sid, "call");

        let update = summary.map(|s| {
            let mut u = VarUpdate::new();
            for w in &s.writes {
                u.insert(w.clone(), Expr::Lit(Val::Unknown));
            }
            u
        });

        emit_simple_transition(
            ctx,
            &t_id,
            TransitionKind::Call,
            &[&stmt.sid],
            input_cp,
            &target_cp,
            BoolExpr::True,
            update,
        );
    }
}

// ── return ──────────────────────────────────────────────────────────────

fn translate_nop(
    ctx: &mut TranslateContext,
    fn_name: &str,
    stmt: &Statement,
    input_cp: &str,
) {
    let plan = plan_transfer(ctx, fn_name, &stmt.sid, &stmt.transfer);

    match plan {
        TransferPlan::Return { target_cp } | TransferPlan::Next { target_cp } => {
            let t_id = tid(fn_name, &stmt.sid, "nop");
            emit_simple_transition(
                ctx,
                &t_id,
                TransitionKind::Sequential,
                &[&stmt.sid],
                input_cp,
                &target_cp,
                BoolExpr::True,
                None,
            );
        }
        TransferPlan::Branch {
            true_tid,
            true_cp,
            false_tid,
            false_cp,
            guard,
        } => {
            emit_branch_transitions(
                ctx,
                &[&stmt.sid],
                input_cp,
                &true_tid,
                &true_cp,
                &false_tid,
                &false_cp,
                guard,
            );
        }
        TransferPlan::Switch { arms } => {
            let switch_var = match &stmt.transfer {
                Transfer::Switch { var, .. } => var.as_str(),
                _ => "",
            };
            emit_switch_transitions(ctx, &[&stmt.sid], input_cp, switch_var, &arms);
        }
    }
}

fn translate_return_op(
    ctx: &mut TranslateContext,
    fn_name: &str,
    stmt: &Statement,
    input_cp: &str,
) {
    let plan = plan_transfer(ctx, fn_name, &stmt.sid, &stmt.transfer);

    match plan {
        TransferPlan::Return { target_cp } => {
            let t_id = tid(fn_name, &stmt.sid, "return");
            emit_simple_transition(
                ctx,
                &t_id,
                TransitionKind::Return,
                &[&stmt.sid],
                input_cp,
                &target_cp,
                BoolExpr::True,
                None,
            );
        }
        TransferPlan::Next { target_cp } => {
            // Op::Return with a non-return transfer (e.g. back edge in a loop).
            let t_id = tid(fn_name, &stmt.sid, "return");
            emit_simple_transition(
                ctx,
                &t_id,
                TransitionKind::Sequential,
                &[&stmt.sid],
                input_cp,
                &target_cp,
                BoolExpr::True,
                None,
            );
        }
        _ => {
            ctx.ensure_return_place(fn_name);
            let target_cp = cp_id(fn_name, "ret");
            let t_id = tid(fn_name, &stmt.sid, "return");
            emit_simple_transition(
                ctx,
                &t_id,
                TransitionKind::Return,
                &[&stmt.sid],
                input_cp,
                &target_cp,
                BoolExpr::True,
                None,
            );
        }
    }
}
