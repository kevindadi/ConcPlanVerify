# Track D — detection capability (no LLM)

- ConcIR binary sha256: `73b5dd64012e5c53c4fb0736a553c8b6d37d6bb99b55766c573136ec9c364794`
- Miri combos: [{'seed': 0, 'preemption_rate': 0.01}, {'seed': 1, 'preemption_rate': 0.05}, {'seed': 2, 'preemption_rate': 0.1}, {'seed': 3, 'preemption_rate': 0.2}, {'seed': 4, 'preemption_rate': 0.5}]
- Lockbud available: False

## Per task

| task | CIR buggy | CIR fixed | Miri buggy | Miri fixed | notes |
| --- | --- | --- | --- | --- | --- |
| P1 | FAIL | PASS | False | False | CIR detects |
| P2 | — | — | — | — | skipped_to_author |
| P3 | — | — | — | — | skipped_to_author |
| P4 | FAIL | PASS | False | False | CIR detects |
| P5 | — | — | — | — | skipped_to_author |
| P6 | — | — | — | — | skipped_to_author |
| P7 | — | — | — | — | skipped_to_author |
| P8 | — | — | — | — | skipped_to_author |
| P9 | — | — | — | — | skipped_to_author |
| rmw-zenoh-998 | FAIL | PASS | None | None | CIR detects |
| dashmap-369 | UNSUPPORTED | None | None | None |  |

## Caveats

- Miri is dynamic and schedule-dependent: a miss is not proof of absence, and is not evidence of safety.
- The deadlock fixtures are lock-order bugs that only manifest on some interleavings; each Miri seed runs one schedule, so a miss here is expected rather than surprising.
- The ConcIR arm consumes human-written CIR, not Rust source; the comparison is capability-level, not same-input.
- Lockbud is unavailable on this machine unless installed and pinned.
