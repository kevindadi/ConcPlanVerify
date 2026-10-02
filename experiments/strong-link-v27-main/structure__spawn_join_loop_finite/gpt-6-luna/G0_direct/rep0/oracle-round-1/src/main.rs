mod cir_trace;
use std::thread;

fn worker() {}

fn main() { cir_trace::init();
    for _ in 0..2 {
        let handle = cir_trace::spawn("handle#87", worker);
        handle.join().expect("worker thread panicked");
    }

    println!("DONE done=1");
 cir_trace::finish();}
