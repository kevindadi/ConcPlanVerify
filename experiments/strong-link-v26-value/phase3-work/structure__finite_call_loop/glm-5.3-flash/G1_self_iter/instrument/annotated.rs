mod cir_trace;
// Auxiliary routine: pure, stateless, runs to completion and returns.
fn helper() {
    // No mutexes, counters, or shared state (R2).
    // All work is local; the call returns when complete (R3).
}

fn main() { cir_trace::init();
    // R1: main task calls the auxiliary routine, then begins the
    // same call sequence again. R3: each call completes before the
    // next starts, guaranteed by sequential execution.
    helper();
    helper();

    // R5: exactly one line of output, then the process exits.
    println!("DONE done=1");
 cir_trace::finish();}
