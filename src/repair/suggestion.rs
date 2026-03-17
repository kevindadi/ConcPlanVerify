use super::report::BugKind;

/// Generate a template-based repair suggestion from the detected bug kind.
pub fn suggestion_for(kind: &BugKind) -> String {
    match kind {
        BugKind::Deadlock { participants } => {
            let mut resources: Vec<&str> = participants
                .iter()
                .flat_map(|p| {
                    p.holding
                        .iter()
                        .map(String::as_str)
                        .chain(std::iter::once(p.waiting_for.as_str()))
                })
                .collect();
            resources.sort();
            resources.dedup();
            let ordered = resources.join(" → ");

            let changes: Vec<String> = participants
                .iter()
                .map(|p| {
                    format!(
                        "函数 {} 中调整 {} 的获取顺序",
                        p.function, p.waiting_for
                    )
                })
                .collect();

            format!(
                "所有函数应按统一顺序获取锁。\n\
                 建议顺序: {ordered}\n\
                 具体修改: {}",
                changes.join("; ")
            )
        }
        BugKind::SignalLoss {
            notifier_tid,
            waiter_tid,
        } => {
            format!(
                "通知者 ({notifier_tid}) 可能在等待者 ({waiter_tid}) 之前执行 notify。\n\
                 修复方案: 在 wait 前用 while 循环检查条件变量。\n\
                 确保即使 notify 已经发生，等待者也能通过条件检查直接跳过 wait。"
            )
        }
        BugKind::ChannelBlock {
            blocked_op,
            channel,
        } => {
            format!(
                "Channel {channel} 的 {blocked_op} 操作可能永远阻塞。\n\
                 修复方案: 确保 send/recv 配对，不要在持有锁时执行可能阻塞的 channel 操作。"
            )
        }
    }
}
