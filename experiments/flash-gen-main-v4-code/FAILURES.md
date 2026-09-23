# flash-gen-main-v4-code — the 5 G3 code-stage failures

Each row is read from the stored cell (`rep<k>/<task>/G3-concir/CELL.json`), the
accepted CIR (`cir_path`), and the last code round's `.rs`.

## 1–4. `sem_release` at event 2 — real deviation (double release)

Cells: `semaphore/acquire_twice_no_release` rep0, rep2;
`semaphore/permit_leak` rep2; `structure/scope_bound_k_workers` rep1.

All four report the same violation:

```
no enabled model step matches sem_release on "main::s" at event 2
```

The accepted CIR (from `flash-gen-main-v2`) is a single acquire/release cycle per
worker, e.g.

```
fn w1: semaphore_acquire s1 main::s; semaphore_release s2 main::s; return s3
```

The LLM's Rust releases the permit **twice**:

```rust
fn w1(s: Arc<Semaphore>) {
    let permit = s.acquire();   // sem_acquire
    drop(permit);               // sem_release  (Permit::drop)
    s.release();                // sem_release  (extra)
}
```

so the observed stream has `acquire, release, release` while the model allows
only `acquire, release`. **Judgement: real deviation** (not a checker bug), but
it is an API footgun: `Semaphore::release(&self)` could be called while a
`Permit` is still held. Fix applied this round: `Semaphore::release` is removed;
release happens on `Permit::drop` or `permit.release()` (consumes the permit),
and the four prompts state "never release the same permit twice". The §4 rerun
regenerates the Rust under the fixed API/prompt.

## 5. `channel/rendezvous_both_send` rep0 — real model failure (build)

Decision `build_failed` in all three rounds. Re-scoring the last round on the
current toolchain still fails to compile:

```
error[E0308]/[E0277]: `cir_trace::spawn("r", move || { r(rx); })` arity and
`println!("DONE done={}", v)` where `v: ()`.
```

**Judgement: real model failure.** The generated Rust does not compile
independently of the harness; the channel family is additionally affected by the
`Mutex<Receiver>` mapping gap (see §3).
