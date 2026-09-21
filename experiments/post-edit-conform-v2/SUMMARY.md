# post-edit-conform-v2 — SUMMARY

- requests: 64/69
- binary sha256: `073129de3a6d378a4e198bf712028c0c950cdf080981fb0073dfb7af5fa7ce5c`

| edit | cells | build_ok | conform PASS | miri | hang | sid_dropped | drift-only-conform |
| --- | --- | --- | --- | --- | --- | --- | --- |
| E1 | 22 | 22 | 22 | 0 | 0 | 0 | 0 |
| E2 | 21 | 20 | 20 | 0 | 0 | 0 | 0 |
| E3 | 21 | 18 | 18 | 0 | 0 | 0 | 0 |

## drift_caught_only_by_conform

| task | arm | edit | reason | sid_dropped |
| --- | --- | --- | --- | --- |

## Edit reality (changed-line counts vs the generated program)

| edit | cells | no-op (0 changed lines) | changed-line range |
| --- | --- | --- | --- |
| E1 logging | 22 | 0 | 6–41 |
| E2 refactor | 20 | 15 | 0–15 |
| E3 optimise | 18 | 11 | 0–123 |

`drift_caught_only_by_conform = 0`: the edits that did change code (all E1, 5 E2,
7 E3) preserved every `cir_trace` sync call (`sid_dropped = 0`, `edited_calls ==
orig_calls`), so the operation-bound event stream is unchanged and conform
correctly passes. E3's "optimisations" did not move lock/unlock calls. No
`sid_dropped`/`unknown_sid` cases occurred.
