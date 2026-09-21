mod cir_trace;
fn main() { cir_trace::init();
    // R1: main task calls an auxiliary routine, then begins the same call sequence again.
    // R2: no shared state between main and auxiliary routine.
    // R3: each auxiliary call runs to completion before the next call starts.
    // R4: every schedule terminates.
    // R5: print exactly "DONE done=1".

    let done = auxiliary();
    let done2 = auxiliary();

    // Both calls complete; combine results without shared mutable state.
    let total = done + done2;
    println!("DONE done={}", total);
 cir_trace::finish();}

fn auxiliary() -> u32 {
    // Pure computation, no shared state, always terminates.
    let mut x = 0u32;
    for i in 0..1 {
        x += i + 1;
    }
    x
}
