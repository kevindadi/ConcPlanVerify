mod cir_trace;
use std::thread;

fn worker() {}

fn main() { cir_trace::init();
    let worker1 = cir_trace::spawn("worker1", worker);
    worker1.join().unwrap();

    let worker2 = cir_trace::spawn("worker2", worker);
    worker2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
