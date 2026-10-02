mod cir_trace;
use std::thread;

fn worker() {
    // R2: worker performs no shared work and shares no state.
}

fn main() { cir_trace::init();
    let h1 = cir_trace::spawn("h1#123", worker);
    h1.join().unwrap();

    let h2 = cir_trace::spawn("h2#184", worker);
    h2.join().unwrap();

    // R6: exact terminal line.
    println!("DONE done=1");
 cir_trace::finish();}
