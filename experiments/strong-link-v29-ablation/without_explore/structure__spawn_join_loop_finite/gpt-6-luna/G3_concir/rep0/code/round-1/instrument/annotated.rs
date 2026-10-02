mod cir_trace;
use std::thread;

fn worker() {}

fn main() { crate::cir_trace::init();
    let handle = crate::cir_trace::spawn("worker#63", move || worker());
    handle.join().unwrap();
    println!("DONE done=1");
 crate::cir_trace::finish();}
