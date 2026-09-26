mod cir_trace;
use std::thread;

fn worker() {
    // Worker performs no shared work and returns immediately.
}

fn main() { cir_trace::init();
    let mut count = 0;
    while count < 1 {
        let handle = cir_trace::spawn("handle", worker);
        handle.join().unwrap();
        count += 1;
    }
    println!("DONE done=1");
 cir_trace::finish();}
