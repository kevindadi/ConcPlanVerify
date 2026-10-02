mod cir_trace;
use std::thread;

fn worker() {}

fn main() { crate::cir_trace::init();
    let w1 = crate::cir_trace::spawn("worker#59", move || worker());
    let _ = w1.join();

    let w2 = crate::cir_trace::spawn("worker#129", move || worker());
    let _ = w2.join();

    let done: i32 = 1;
    println!("DONE done={}", done);
 crate::cir_trace::finish();}
