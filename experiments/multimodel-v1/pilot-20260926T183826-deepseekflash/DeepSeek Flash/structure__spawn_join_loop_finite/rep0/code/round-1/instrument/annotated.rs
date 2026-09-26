mod cir_trace;
use std::thread;

fn worker() {
}

fn main() { cir_trace::init();
    let w1 = cir_trace::spawn("w1", worker);
    w1.join().unwrap();

    let w2 = cir_trace::spawn("w2", worker);
    w2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
