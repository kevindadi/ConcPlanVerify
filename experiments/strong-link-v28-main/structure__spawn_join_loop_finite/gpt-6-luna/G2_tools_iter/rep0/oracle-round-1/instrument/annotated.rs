mod cir_trace;
use std::thread;

fn worker() {}

fn main() { cir_trace::init();
    let first = cir_trace::spawn("first#62", worker);
    first.join().expect("worker panicked");

    let second = cir_trace::spawn("second#147", worker);
    second.join().expect("worker panicked");

    println!("DONE done=1");
 cir_trace::finish();}
