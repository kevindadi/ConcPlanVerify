mod cir_trace;
use std::thread;

/// CIR function `main::worker` — performs no shared work and shares no state.
fn worker() {}

/// CIR function `main::main`.
fn main() { cir_trace::init();
    // s1: spawn main::worker as h1
    let h1 = cir_trace::spawn("worker#207", || worker());
    // s2: join h1
    h1.join().unwrap();

    // s3: spawn main::worker as h2
    let h2 = cir_trace::spawn("worker#328", || worker());
    // s4: join h2
    h2.join().unwrap();

    // s5: return — print the required terminal line.
    println!("DONE done=1");
 cir_trace::finish();}
