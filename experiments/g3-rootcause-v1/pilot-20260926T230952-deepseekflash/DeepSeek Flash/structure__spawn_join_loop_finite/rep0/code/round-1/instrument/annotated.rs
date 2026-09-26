mod cir_trace;
use std::thread;

fn worker() {
}

fn main() { cir_trace::init();
    let mut done = 0;

    let h1 = cir_trace::spawn("h1", worker);
    h1.join().unwrap();

    done = 1;

    let h2 = cir_trace::spawn("h2", worker);
    h2.join().unwrap();

    println!("DONE done={}", done);
 cir_trace::finish();}
