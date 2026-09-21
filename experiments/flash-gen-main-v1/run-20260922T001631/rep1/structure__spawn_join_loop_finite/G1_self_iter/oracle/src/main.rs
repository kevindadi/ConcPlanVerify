mod cir_trace;
use std::thread;

fn main() { cir_trace::init();
    for _ in 0..1 {
        let handle = cir_trace::spawn("handle", || {
            // R2: worker performs no shared work
        });
        // R3: wait exactly once before next iteration
        handle.join().unwrap();
    }
    println!("DONE done=1");
 cir_trace::finish();}
