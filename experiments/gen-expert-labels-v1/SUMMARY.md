# gen-expert-labels-v1 — rubric v3 summary

Annotated (sha-dedup, G3 then G0): **40** of 179 unique accepted Rust; 48 `not_run` (budget 40). Requests 40.

- `bug_present=yes`: **2**; no: 38; unsure: 0
- agent says bug but monitor/conform did **not** flag (tool-missed): **2**
- annotated cells with at least one `Ri` unsatisfied per the agent: 11

## Tool-missed bugs (agent yes, tools clean)

| arm | task | sha[12] | agent evidence |
| --- | --- | --- | --- |
| G3_concir | lock-order/cycle_3lock | `7f565d54edd3` | t1 lines 11-12 (a then b), t2 lines 18-19 (b then c), t3 lines 25-26 (a then c): inconsistent lock o |
| G3_concir | lock-order/cycle_3lock | `68216e3ecd00` | t3 lines: `let _ga = a3.lock().unwrap(); let _gc = c3.lock().unwrap();` acquire a then c, while t1 a |

Both are `lock-order/cycle_3lock`: the proxy reads a three-lock cycle in the
accepted Rust although `conform`/`monitor` accepted it. Human review is the next
step (`HUMAN_REVIEW_QUEUE_GEN.md`).
