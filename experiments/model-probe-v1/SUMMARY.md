# model-probe-v1 — not run (no second model available)

The endpoint `https://api.deepseek.com` maps both `deepseek-chat` and
`deepseek-reasoner` to `deepseek-flash`: a probe request returns
`"model": "deepseek-flash"`. There is therefore **no distinct non-Flash model**
to probe, and the pinned client's identity check would reject a response whose
`model` is not `deepseek-flash`.

Result: §5 is a **stop point** (K-4b not done — endpoint has no second model).
No requests spent. The generalization claim rests on the 10-task main batch (§4)
and the second reference tasks, not on a second model.
