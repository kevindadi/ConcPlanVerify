use std::fmt::Write;

use super::report::{BugKind, BugReport, DeadlockParticipant};
use super::suggestion::suggestion_for;

/// Render a bug report as human-readable text (also suitable as LLM input).
pub fn render_text(report: &BugReport) -> String {
    let mut out = String::new();

    write_header(&mut out, report);
    write_trace(&mut out, report);
    write_bug_details(&mut out, report);
    write_suggestion(&mut out, report);

    out
}

/// Render a full LLM repair prompt containing the original CIR, the bug
/// report, and repair instructions.
pub fn render_repair_prompt(report: &BugReport, original_cir_json: &str) -> String {
    let mut out = String::new();

    writeln!(out, "# 并发 Bug 修复请求").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "## 原始 CIR").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "```json").unwrap();
    writeln!(out, "{original_cir_json}").unwrap();
    writeln!(out, "```").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "## 检测到的 Bug").unwrap();
    writeln!(out).unwrap();
    write!(out, "{}", render_text(report)).unwrap();
    writeln!(out).unwrap();
    writeln!(out, "## 修复指导").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "### Bug 类型:{}", report.kind.name()).unwrap();
    writeln!(out).unwrap();
    writeln!(out, "{}", suggestion_for(&report.kind)).unwrap();
    writeln!(out).unwrap();
    write_repair_constraints(&mut out);
    write_common_patterns(&mut out);
    writeln!(out, "## 输出要求").unwrap();
    writeln!(out).unwrap();
    writeln!(
        out,
        "请输出修复后的**完整 CIR JSON**,不要省略任何函数或资源定义."
    )
    .unwrap();

    out
}

fn write_header(out: &mut String, report: &BugReport) {
    writeln!(out, "BUG: {}", report.summary).unwrap();
    writeln!(out).unwrap();
}

fn write_trace(out: &mut String, report: &BugReport) {
    if report.trace.is_empty() {
        return;
    }
    writeln!(out, "TRACE ({} steps):", report.trace.len()).unwrap();
    for (i, step) in report.trace.iter().enumerate() {
        let sids = if step.anchor_sids.is_empty() {
            step.transition_id.clone()
        } else {
            step.anchor_sids.join(", ")
        };
        writeln!(out, "  {}. [{}] {}", i + 1, sids, step.description).unwrap();
    }
    writeln!(out).unwrap();
}

fn write_bug_details(out: &mut String, report: &BugReport) {
    match &report.kind {
        BugKind::Deadlock { participants } => {
            writeln!(out, "DEADLOCK:").unwrap();
            for p in participants {
                write_participant(out, p);
            }
        }
        BugKind::SignalLoss {
            notifier_tid,
            waiter_tid,
        } => {
            writeln!(out, "SIGNAL LOSS:").unwrap();
            writeln!(out, "  notifier: {notifier_tid}").unwrap();
            writeln!(out, "  waiter blocked at: {waiter_tid}").unwrap();
        }
        BugKind::ChannelBlock {
            blocked_op,
            channel,
        } => {
            writeln!(out, "CHANNEL BLOCK:").unwrap();
            writeln!(out, "  channel: {channel}, blocked on: {blocked_op}").unwrap();
        }
    }
    writeln!(out).unwrap();
    if !report.final_marking_summary.is_empty() {
        writeln!(out, "FINAL STATE: {}", report.final_marking_summary).unwrap();
        writeln!(out).unwrap();
    }
}

fn write_participant(out: &mut String, p: &DeadlockParticipant) {
    let holding = if p.holding.is_empty() {
        "(none)".to_string()
    } else {
        format!("[{}]", p.holding.join(", "))
    };
    writeln!(
        out,
        "  {}: 持有 {holding}, 等待 {} (blocked at {})",
        p.function, p.waiting_for, p.blocked_at_sid
    )
    .unwrap();
}

fn write_suggestion(out: &mut String, report: &BugReport) {
    writeln!(out, "SUGGESTION: {}", suggestion_for(&report.kind)).unwrap();
}

fn write_repair_constraints(out: &mut String) {
    writeln!(out, "### 修复约束").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "1. **最小修改原则**:只修改导致 bug 的函数,不要重写整个程序").unwrap();
    writeln!(out, "2. **保持 sid 格式**:使用 \"s\" + 数字,函数内唯一").unwrap();
    writeln!(
        out,
        "3. **保持资源不变**:不要新增或删除资源定义,除非修复方案确实需要"
    )
    .unwrap();
    writeln!(out, "4. **transfer 显式跳转**:每条 next 必须带目标 sid").unwrap();
    writeln!(out, "5. **锁配对**:每个 lock 必须有对应的 drop").unwrap();
    writeln!(
        out,
        "6. **Condvar 惯用法**:wait 前用 while 循环检查条件变量"
    )
    .unwrap();
    writeln!(out).unwrap();
}

fn write_common_patterns(out: &mut String) {
    writeln!(out, "### 常见修复模式").unwrap();
    writeln!(out).unwrap();

    writeln!(out, "#### 死锁 → 统一锁顺序").unwrap();
    writeln!(out).unwrap();
    writeln!(
        out,
        "所有函数按相同顺序获取锁.例如全局约定 mtx_a → mtx_b → mtx_c."
    )
    .unwrap();
    writeln!(out).unwrap();

    writeln!(out, "#### 信号丢失 → while 循环保护 wait").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "先读条件变量,true 则跳过 wait;false 则 wait 后回到条件检查.").unwrap();
    writeln!(out).unwrap();

    writeln!(out, "#### Channel + Mutex 死锁 → 不在持锁时做阻塞操作").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "将 recv/send 移到 lock/drop 之外.").unwrap();
    writeln!(out).unwrap();
}
