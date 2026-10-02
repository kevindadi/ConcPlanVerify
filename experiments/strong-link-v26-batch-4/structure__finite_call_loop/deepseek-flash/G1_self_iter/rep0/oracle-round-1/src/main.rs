mod cir_trace;
use std::thread;

fn helper() {
    // Auxiliary routine: no shared state, no output.
}

fn main() { cir_trace::init();
    // First auxiliary call
    let handle = cir_trace::spawn("handle#146", helper);
    handle.join().expect("helper thread panicked");

    // Second auxiliary call
    let handle = cir_trace::spawn("handle#268", helper);
    handle.join().expect("helper thread panicked");

    println!("DONE done=1");
 cir_trace::finish();}
