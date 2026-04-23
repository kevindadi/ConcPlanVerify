mod condvar;
mod context;
mod control_flow;
mod expr_parser;
mod fn_summary;
pub(crate) mod goals;
mod operation;
mod resource;

pub use goals::translate_goals;

use crate::error::TranslateError;
use context::{TranslateContext, cp_id};
use cvn::net::CvnNet;

/// Translate a CIR program into a CVN network.
///
/// This is the single public entry point of the translator.
/// Internally it executes three phases in order:
///   1. Resource scanning  — generate resource places, initial marking, and variable store
///   2. Function body translation — generate control places, transitions, and arcs
///   3. FnSummary translation — generate atomic transitions for un-modeled functions
pub fn translate(program: &cir::ast::Program) -> Result<CvnNet, Vec<TranslateError>> {
    let mut ctx = TranslateContext::new();

    // ── Input validation (T0xx) ─────────────────────────────────────────

    let entry_fn = program.functions.iter().find(|f| f.name == program.entry);
    if entry_fn.is_none() {
        ctx.push_error(TranslateError::MissingEntry(program.entry.clone()));
    }
    if let Some(ef) = entry_fn {
        if ef.body.is_empty() {
            ctx.push_error(TranslateError::EmptyEntryBody(program.entry.clone()));
        }
    }

    // Validate spawn/join/call targets.
    validate_function_references(&mut ctx, program);

    if ctx.has_errors() {
        return Err(ctx.errors);
    }

    // ── Phase 0: Index FnSummaries so Phase 2 can look them up ──────────

    fn_summary::index_fn_summaries(&mut ctx, &program.fn_summaries);

    // ── Phase 1: Resource scanning ──────────────────────────────────────

    resource::scan_resources(&mut ctx, program);

    if ctx.has_errors() {
        return Err(ctx.errors);
    }

    // ── Phase 2: Function body translation ──────────────────────────────

    // Resolve "s_first" aliases: for each function, if the first statement's
    // sid is not "s_first", we need the spawn output arc to point to the
    // actual first sid. We handle this by mapping cp(fn, "s_first") to the
    // real first sid's place via ensuring they are the same place.
    // Actually, we just set an initial token on the real first sid for the
    // entry function, and for spawned functions the spawn transition
    // outputs to cp(fn, "s_first"). We need cp(fn, "s_first") to be the
    // actual first statement's place.
    //
    // Strategy: before translation, register cp(fn, "s_first") as an alias
    // by ensuring the first sid's place is the one used. We'll pre-create
    // "s_first" places and then in Phase 2, the spawn output will connect
    // to them. But we also need the actual first statement to be that same
    // place. Instead, let's just fix the spawn translation to use the real
    // first sid.

    // Build a map: fn_name → first_sid for spawned functions.
    operation::translate_functions(&mut ctx, &program.functions);

    // Fix spawn aliases: cp(fn, "s_first") should be the actual first sid.
    // Since the spawn transition outputs to cp(fn, "s_first"), we need to
    // ensure this place gets a token when the actual first place does.
    // The simplest fix: we already called ensure_control_place(fn, "s_first")
    // in spawn translation. Now add a silent Sequential transition
    // s_first → actual_first_sid if they differ.
    for func in &program.functions {
        if let Some(first_stmt) = func.body.first() {
            if first_stmt.sid != "s_first"
                && ctx
                    .control_places
                    .contains(&(func.name.clone(), "s_first".to_string()))
            {
                let from = cp_id(&func.name, "s_first");
                let to = cp_id(&func.name, &first_stmt.sid);
                let bridge_tid = format!("{}_s_first_bridge", func.name);
                ctx.add_transition(
                    &bridge_tid,
                    cvn::model::TransitionKind::Sequential,
                    &[&first_stmt.sid],
                );
                ctx.add_input_arc(
                    &from,
                    &bridge_tid,
                    1,
                    cvn::model::BoolExpr::True,
                );
                ctx.add_output_arc(&bridge_tid, &to, 1, None);
            }
        }
    }

    // ── Set initial marking for entry function ──────────────────────────

    if let Some(ef) = entry_fn {
        if let Some(first_stmt) = ef.body.first() {
            let entry_cp = cp_id(&ef.name, &first_stmt.sid);
            ctx.set_initial_tokens(&entry_cp, 1);
        }
    }

    // ── Finalize ────────────────────────────────────────────────────────

    ctx.finish()
}

/// Validate that all spawn/join/call targets reference existing functions or summaries.
fn validate_function_references(ctx: &mut TranslateContext, program: &cir::ast::Program) {
    let fn_names: std::collections::HashSet<&str> = program
        .functions
        .iter()
        .map(|f| f.name.as_str())
        .chain(program.fn_summaries.iter().map(|s| s.name.as_str()))
        .collect();

    for func in &program.functions {
        for stmt in &func.body {
            if let Some(target) = stmt.op.target_name() {
                if !fn_names.contains(target) {
                    ctx.push_error(TranslateError::UnknownFunction(target.to_string()));
                }
            }
        }
    }
}
