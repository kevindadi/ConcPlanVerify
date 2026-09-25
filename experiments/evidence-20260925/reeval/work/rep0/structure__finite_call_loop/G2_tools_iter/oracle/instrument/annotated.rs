mod cir_trace;
use std::thread;

fn helper() {
    // Auxiliary routine: performs its work and returns.
}

fn main() { cir_trace::init();
    // R1: main task calls the auxiliary routine, then begins the same call sequence again.
    // R3: each auxiliary call runs to completion before the next call starts.
    let h1 = cir_trace::spawn("h1", helper);
    h1.join().unwrap();

    let h2 = cir_trace::spawn("h2", helper);
    h2.join().unwrap();

    // R5: print exactly the required line and exit.
    println!("DONE done=1");
 cir_trace::finish();}
