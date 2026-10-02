mod cir_trace;
use std::thread;

fn worker() {}

fn main() { crate::cir_trace::init();
    let h1 = crate::cir_trace::spawn("worker#59", move || worker());
    h1.join().expect("worker h1 panicked");

    let h2 = crate::cir_trace::spawn("worker#150", move || worker());
    h2.join().expect("worker h2 panicked");

    println!("DONE done=1");
 crate::cir_trace::finish();}
