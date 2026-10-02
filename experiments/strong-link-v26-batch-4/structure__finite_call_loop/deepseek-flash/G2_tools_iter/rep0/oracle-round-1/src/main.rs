mod cir_trace;
use std::thread;

fn helper() {
    // The auxiliary routine does not share any state with the main task.
}

fn main() { cir_trace::init();
    // First call of the auxiliary routine.
    let h1 = cir_trace::spawn("helper#178", || helper());
    h1.join().unwrap();

    // Begin the same call sequence again: call the auxiliary routine a second time.
    let h2 = cir_trace::spawn("helper#329", || helper());
    h2.join().unwrap();

    let done = 1;
    println!("DONE done={}", done);
 cir_trace::finish();}
