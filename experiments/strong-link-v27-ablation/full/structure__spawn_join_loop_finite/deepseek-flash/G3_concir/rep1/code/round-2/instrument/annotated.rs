mod cir_trace;
use std::thread;

fn worker() {}

fn main() { cir_trace::init();
    let h1 = cir_trace::spawn("worker#59", || worker());
    h1.join().unwrap();

    let h2 = cir_trace::spawn("worker#125", || worker());
    h2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
