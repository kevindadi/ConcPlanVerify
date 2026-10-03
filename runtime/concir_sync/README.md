# concir_sync

Standard-library-only counting semaphore used by the ConcPlanVerify generation
harness. Generated cargo projects depend on it by path so the model can write
`use concir_sync::Semaphore;` without declaring a module.

- `Semaphore::new(n)` / `new_named(name, n)` (returns `Arc<Self>`)
- RAII: `acquire()` -> `Permit`. Dropping the permit, or `permit.release()`, returns exactly one permit. `try_acquire()` is the same RAII shape. There is no `Semaphore::release`.
- Explicit count: `release_count(n)` adds `n` permits. `acquire_count(n)` consumes `n` permits and does not return them when the call ends. `n <= 0` is `SemCountError::NonPositive`. Adding past `i64::MAX` is `Overflow`.

A count of 2 is one operation. It is not rewritten into two count-1 events. RAII events omit `n` and mean one permit. Explicit events are recorded with `n` when a count recorder is installed. If that recorder is absent, an explicit count other than the call itself is not invented as extra log lines.

`acquire`/`release` call a recorder installed via `set_recorder`; explicit counts call `set_count_recorder`. The generated `cir_trace` runtime installs both. The crate contains no tracing itself and builds standalone. Historical RAII programs keep the old drop-to-return meaning.
