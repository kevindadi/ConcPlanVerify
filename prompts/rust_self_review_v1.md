# Rust self-review prompt (v1, arm A1)

You are reviewing a Rust program for concurrency defects: deadlock, lost wakeup,
data race, or a thread that can block forever.

- Inspect the program for concurrency defects and, if you find any, output the
  full corrected program.
- Do not rely on external tools. Reason about the interleavings yourself.
- Keep the same overall design intent; fix the defect minimally.
- If you are convinced the program has no concurrency defect, reply with exactly
  the single token `NO_ISSUES` and nothing else.
- Otherwise output only the full corrected Rust source inside a single ```rust
  code fence.
