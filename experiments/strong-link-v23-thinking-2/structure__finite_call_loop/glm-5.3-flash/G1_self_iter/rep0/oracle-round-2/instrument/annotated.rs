mod cir_trace;
// Auxiliary routine.
//
// R2: the spawned task captures nothing from `main`'s environment — there are
// no mutexes, counters, atomics, or statics shared between the main task and
// the helper task, so no two tasks ever contend for a resource.
//
// R3: `join()` guarantees this auxiliary call runs to completion before
// `helper` returns, so the caller cannot start its next call early.
//
// R4: the helper task is joined unconditionally on every call, so every
// schedule/interleaving terminates; there is no channel, lock, or loop that
// could deadlock or spin.
fn helper() {
    let task = cir_trace::spawn("black_box#603", || {
        // Purely local work: nothing is read from or written to any
        // state owned by the main task.
        let local: u32 = 1 + 1;
        std::hint::black_box(local);
    });
    task.join().expect("helper task must not panic");
}

fn main() { cir_trace::init();
    // R1: call the auxiliary routine, then begin the same call sequence again.
    helper();
    helper();

    // R2: `done` is a local variable of the main task only; it is never
    // shared with (or visible to) `helper`, so it is not shared state.
    let done = 1;

    // R5: exactly one line is printed, then the process exits.
    println!("DONE done={}", done);
 cir_trace::finish();}
