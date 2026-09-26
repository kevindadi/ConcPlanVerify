mod cir_trace;
use std::thread;

fn worker() {
    // no operations
}

fn main() { cir_trace::init();
    let h1 = cir_trace::spawn("worker", || worker());
    h1.join().unwrap();

    let h2 = cir_trace::spawn("worker", || worker());
    h2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
