# Conformance v4 — PROTOCOL (frozen 2026-09-18g)

Offline conformance with the composite-rendezvous rule (channel ops are
attempt-events, `ev` before the call), plus a bounded Flash hole-fill and a
`A3_free` ablation on `structure/worker_with_payload`.

- Offline: 50 native + 8 miri seeds per case; coverage per `(function, sid)`.
- Live (<=6 requests): `skeleton_fill` (one Flash request fills the codegen hole)
  and `A3_free` (one Flash request writes the whole annotated Rust; the prompt
  requires `cir_trace::finish()`).
- Channel rule: cap=0 rendezvous is one model step that consumes two completion
  events in either order; both sides' completions are the model's completion
  steps, so `ev` is emitted *before* the call (attempt semantics).
