mod cir_trace;
// Auxiliary routine: `helper`.
// It touches no mutexes, counters, globals, or any other shared state,
// so no two tasks can ever contend for a resource (R2).
// It always runs to completion and terminates (R4).
fn helper() {
    // No shared state, no blocking, no loops: trivially terminates.
}

fn main() { cir_trace::init();
    // R1: the main task calls the auxiliary routine...
    {
        let handle = cir_trace::spawn("handle#395", helper);
        // R3: wait until this call has run to completion
        // before starting the next call.
        handle.join().expect("helper task panicked");
    }

    // R1: ...and then begins the same call sequence again.
    {
        let handle = cir_trace::spawn("handle#671", helper);
        // R3: each auxiliary call completes before the next one starts.
        handle.join().expect("helper task panicked");
    }

    // R5: print exactly the required line, then exit.
    println!("DONE done=1");
 cir_trace::finish();}
