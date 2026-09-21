mod cir_trace;
use std::thread;

fn auxiliary() {
    // Auxiliary routine: no shared state, no contention.
    // It simply performs a small amount of local work.
    let mut x: u64 = 0;
    for i in 0..1000u64 {
        x = x.wrapping_add(i);
    }
    // Prevent the compiler from optimizing the work away.
    std::hint::black_box(x);
}

fn main() { cir_trace::init();
    // R1: main task calls the auxiliary routine, then begins the same call
    // sequence again.
    // R3: each auxiliary call runs to completion before the next call starts.
    // We model the auxiliary routine as a spawned thread that is joined,
    // ensuring completion before the next call begins.

    // First call sequence.
    let handle1 = cir_trace::spawn("handle1", auxiliary);
    handle1.join().expect("auxiliary thread 1 panicked");

    // Second call sequence.
    let handle2 = cir_trace::spawn("handle2", auxiliary);
    handle2.join().expect("auxiliary thread 2 panicked");

    // R5: print exactly the required line and exit.
    println!("DONE done=1");
 cir_trace::finish();}
