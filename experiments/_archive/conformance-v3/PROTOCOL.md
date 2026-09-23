# Conformance v3 — PROTOCOL (frozen 2026-09-18f)

Offline conformance under the new `(function, sid)` coverage口径 and per-seed
Miri traces, plus a bounded Flash **hole-fill** and an `A3_free` ablation on
`structure/worker_with_payload` (whose `compute` nobody gives codegen a HOLE).

## Offline

- Cases: the four v2 cases (`abba_2lock`, `lost_wakeup`, `permit_leak`,
  `scope_bound`), the channel fixed cases, and `real-cases/rmw-zenoh-998`.
- `traces_total = 50 native + 8 miri seeds`; coverage per `(function, sid)`.
- Timeout is the `timeout` column with `hang_suspect`; a violation is recorded as
  is, never excused.

## Live (≤6 requests)

- `skeleton_fill`: one Flash request fills the codegen hole(s); lint + 50 native
  + 16 miri traces; conformant/violation/coverage reported.
- `A3_free`: one Flash request writes the whole annotated Rust for the same CIR;
  build + the same traces; conformant/violation/coverage reported.
- Model `deepseek-flash`, thinking disabled, temperature 0, `max_tokens=4096`,
  timeout 90 s, SDK retries 0 + <=1 transient. Budget file shared, restart never
  resets.

## Known deviation

Codegen maps channels to the emitted `cir_trace::Channel`. For zero-capacity
rendezvous the runtime completion order (receiver-first) differs from the
reference model's (second-arriver) attribution, so the channel cases are reported
as violations rather than excused. This is a codegen/model ordering gap, recorded
honestly.
