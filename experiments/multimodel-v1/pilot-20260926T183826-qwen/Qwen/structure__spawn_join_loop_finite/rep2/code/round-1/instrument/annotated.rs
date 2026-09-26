mod cir_trace;
use std::thread;

fn worker() {
    // Worker performs no shared work and shares no mutexes or counters.
}

fn main() { cir_trace::init();
    let handle = cir_trace::spawn("handle", worker);
    handle.join().unwrap();
    println!("DONE done=1");
 cir_trace::finish();}
