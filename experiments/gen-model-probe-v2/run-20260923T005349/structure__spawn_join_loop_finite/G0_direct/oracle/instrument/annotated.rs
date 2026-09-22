mod cir_trace;
use std::thread;

/// The worker role.
///
/// R2: The worker performs no shared work and shares no mutexes or
/// counters with the main task; all of its state is purely local.
fn worker() {
    let mut local: u64 = 0;
    for i in 0..10_000u64 {
        local = local.wrapping_mul(31).wrapping_add(i);
    }
    // Keep the local computation observable to the compiler without
    // introducing any shared state or output.
    std::hint::black_box(local);
}

fn main() { cir_trace::init();
    // R1/R3: Main repeatedly starts a worker and waits for it exactly
    // once before the next worker is started.
    //
    // R4/R5: Each worker runs finite local work and always returns, so
    // every `join` completes and every schedule terminates.
    for _ in 0..3 {
        let handle = cir_trace::spawn("handle", worker);
        handle.join().expect("worker always finishes");
    }

    // R6: Print exactly this line and then exit.
    println!("DONE done=1");
 cir_trace::finish();}
