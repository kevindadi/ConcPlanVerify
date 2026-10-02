mod cir_trace;
use std::thread;

fn worker() {}

fn main() { cir_trace::init();
    let mut done: i32 = 0;

    let h1 = cir_trace::spawn("h1#87", worker);
    h1.join().unwrap();
    done = 1;

    let h2 = cir_trace::spawn("h2#162", worker);
    h2.join().unwrap();
    done = 1;

    println!("DONE done={}", done);
 cir_trace::finish();}
