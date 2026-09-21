mod cir_trace;
fn main() { cir_trace::init();
    // R1: main task calls an auxiliary routine, then begins the same call sequence again.
    // R2: no shared state between main and auxiliary routine.
    // R3: each auxiliary call runs to completion before the next call starts.
    // R4: every schedule terminates.
    // R5: print exactly "DONE done=1".

    let handle1 = cir_trace::spawn("handle1", auxiliary);
    handle1.join().unwrap();

    let handle2 = cir_trace::spawn("handle2", auxiliary);
    handle2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}

fn auxiliary() {
    // Auxiliary routine: no shared state, just does some local work.
    let mut local = 0u64;
    for i in 0..1000 {
        local = local.wrapping_add(i);
    }
    // Prevent optimization from removing the work.
    std::hint::black_box(local);
}
