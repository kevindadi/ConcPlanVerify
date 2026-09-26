mod cir_trace;
use std::thread;

fn worker() {
}

fn main() { cir_trace::init();
    let h1 = cir_trace::spawn("h1", worker);
    h1.join().unwrap();

    let h2_kept = cir_trace::spawn("h2_kept", worker);
    h2_kept.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
