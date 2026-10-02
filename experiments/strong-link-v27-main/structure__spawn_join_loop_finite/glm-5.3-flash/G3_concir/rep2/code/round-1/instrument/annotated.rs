mod cir_trace;
use std::thread;

fn worker() {}

fn main() { cir_trace::init();
    let h1 = cir_trace::spawn("h1#59", worker);
    h1.join().expect("worker h1 panicked");

    let h2 = cir_trace::spawn("h2#140", worker);
    h2.join().expect("worker h2 panicked");

    println!("DONE done=1");
 cir_trace::finish();}
