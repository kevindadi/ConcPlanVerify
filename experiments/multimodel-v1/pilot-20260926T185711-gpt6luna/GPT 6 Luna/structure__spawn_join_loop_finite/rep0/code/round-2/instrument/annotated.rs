mod cir_trace;
use std::thread;

fn worker() {}

fn main() { cir_trace::init();
    let worker1 = cir_trace::spawn("worker", || worker());
    worker1.join().unwrap();

    let worker2 = cir_trace::spawn("worker", || worker());
    worker2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
