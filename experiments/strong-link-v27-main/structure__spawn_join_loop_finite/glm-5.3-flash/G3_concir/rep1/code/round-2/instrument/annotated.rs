mod cir_trace;
use std::thread;

fn worker() {
    // worker performs no shared work; it just returns
}

fn main() { cir_trace::init();
    // First start-and-wait cycle
    let h1 = cir_trace::spawn("worker#149", || {
        worker();
    });
    h1.join().expect("worker h1 panicked");

    // Second start-and-wait cycle
    let h2 = cir_trace::spawn("worker#287", || {
        worker();
    });
    h2.join().expect("worker h2 panicked");

    println!("DONE done=1");
 cir_trace::finish();}
