use cir::ast::FnSummary;

use super::context::TranslateContext;

/// Phase 3: Index FnSummary entries into the context.
///
/// The actual transition generation happens during Phase 2 when `Op::Call(f)`
/// encounters a function name that has a summary (see `operation::translate_call`).
/// This phase merely populates the `fn_summary_map` so that Phase 2 can look
/// up summaries.
pub(crate) fn index_fn_summaries(ctx: &mut TranslateContext, summaries: &[FnSummary]) {
    for s in summaries {
        ctx.fn_summary_map.insert(s.name.clone(), s.clone());
    }
}
