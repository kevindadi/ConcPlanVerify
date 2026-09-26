mod cir_trace;
use std::thread;

fn worker() {
    // no operations
}

fn main() { cir_trace::init();
    let handle = cir_trace::spawn("handle", worker);
    handle.join().unwrap();
    println!("DONE done=1");
 cir_trace::finish();}
