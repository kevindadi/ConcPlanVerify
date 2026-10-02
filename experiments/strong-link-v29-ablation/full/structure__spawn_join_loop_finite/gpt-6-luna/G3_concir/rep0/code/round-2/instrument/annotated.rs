mod cir_trace;
use std::thread;

fn worker() {}

fn main() { crate::cir_trace::init();
    let worker1 = crate::cir_trace::spawn("worker#64", move || worker());
    worker1.join().unwrap();

    let worker2 = crate::cir_trace::spawn("worker#145", move || worker());
    worker2.join().unwrap();

    println!("DONE done=1");
 crate::cir_trace::finish();}
