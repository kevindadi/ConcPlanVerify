mod cir_trace;
use std::thread;

fn helper() {
    // No shared state; each call runs independently.
}

fn main() { cir_trace::init();
    let handle = cir_trace::spawn("handle#118", helper);
    handle.join().unwrap();

    let handle = cir_trace::spawn("handle#187", helper);
    handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
