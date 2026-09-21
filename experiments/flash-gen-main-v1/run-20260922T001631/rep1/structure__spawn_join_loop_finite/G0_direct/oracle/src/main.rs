mod cir_trace;
use std::thread;

fn main() { cir_trace::init();
    for _ in 0..1 {
        let handle = cir_trace::spawn("handle", || {
            // Worker performs no shared work.
        });
        handle.join().unwrap();
    }
    println!("DONE done=1");
 cir_trace::finish();}
