# CIR Specification Reference

> See [`../cir/README.md`](../cir/README.md) for the CIR library documentation.

The CIR (Concurrency Intermediate Representation) is defined in the `ceir` crate.
Key types are in `cir::ast`:

- `Program` — top-level program with resources, functions, summaries
- `Resource` — sync primitives (Mutex, RwLock, Condvar, Semaphore, Channel) and variables (Var, Atomic)
- `Function` — function with a body of `Statement`s
- `Statement` — `{ sid, op, transfer }`
- `Op` — operation (ResOp, Spawn, Join, Call, Return)
- `Transfer` — control flow (Next, Branch, Switch, Return)
- `FnSummary` — abstract summary for un-modeled functions
