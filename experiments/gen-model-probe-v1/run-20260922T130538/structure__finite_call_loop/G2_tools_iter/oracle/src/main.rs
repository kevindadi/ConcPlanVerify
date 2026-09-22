mod cir_trace;
use std::thread;

/// Auxiliary routine: performs purely local work.
///
/// R2: It shares no mutexes, counters, or any other state with the
/// main task, so no two tasks ever contend for a resource.
fn auxiliary(round: u64) {
    let mut acc: u64 = 0;
    for i in 0..10_000u64 {
        acc = acc.wrapping_add(i).wrapping_add(round);
    }
    // Consume the result without producing any output.
    std::hint::black_box(acc);
}

fn main() { cir_trace::init();
    // R1/R3: The main task calls the auxiliary routine; the call runs
    // to completion (the worker thread is joined) before the calling
    // task begins the same call sequence again.
    let first = cir_trace::spawn("first", || auxiliary(1));
    first.join().expect("auxiliary worker must not panic");

    let second = cir_trace::spawn("second", || auxiliary(2));
    second.join().expect("auxiliary worker must not panic");

    // R4: No locks and no unbounded loops, so every schedule terminates.
    // R5: Print exactly this line and then exit.
    println!("DONE done=1");
 cir_trace::finish();}
