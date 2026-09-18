# Pattern tasks

Per-task contents: `spec.md` (natural-language design intent), `buggy.rs` /
`fixed.rs` (legacy Rust references), `buggy.cir.json` / `fixed.cir.json`
(modular CIR ground truth), `contract.json`, `ground_truth.json`, and
`legacy_brief.md` (the legacy canonical requirement, kept for authoring only —
it can reveal the defect and is never the live generation input).

Regenerate the manifest after authoring: `python3 benchmarks/build_patterns.py`.

| task | paper pattern | bug kind | status |
| --- | --- | --- | --- |
| P1 | Two-mutex deadlock | Deadlock | ready |
| P2 | Condvar signal loss | SignalLoss | to_author |
| P3 | Channel + mutex DL | Deadlock / ChannelBlock | to_author |
| P4 | Three-lock circular | Deadlock | ready |
| P5 | Partial deadlock | GoalUnreachable | to_author |
| P6 | Dual condvar cross | SignalLoss / Deadlock | to_author |
| P7 | Semaphore throttle | none (baseline) | to_author |
| P8 | CAS contention | none (baseline) | to_author |
| P9 | FnSummary propagation | none (baseline) | to_author |

`to_author` means the modular CIR, contract and behavioral test still have to be
authored and validated; the Rust references and ground truth already exist. These
tasks are excluded from live batches and reported as excluded.
