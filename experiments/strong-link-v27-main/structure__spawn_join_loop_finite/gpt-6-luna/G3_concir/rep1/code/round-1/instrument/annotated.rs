mod cir_trace;
use std::thread;

fn worker() {}

fn main() { cir_trace::init();
    let worker1 = cir_trace::spawn("worker1#64", worker);
    worker1.join().unwrap();

    let worker2 = cir_trace::spawn("worker2#135", worker);
    worker2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
