mod cir_trace;
use std::thread;

/// Auxiliary routine: uses only local state, so it shares nothing
/// with the calling task and always terminates.
fn helper() {
    let mut local_sum: u64 = 0;
    for i in 0..10u64 {
        local_sum = local_sum.wrapping_add(i);
    }
    let _ = local_sum;
}

fn main() { cir_trace::init();
    // First call sequence: run the auxiliary routine to completion.
    let first = cir_trace::spawn("first", helper);
    first.join().expect("helper thread panicked");

    // Begin the same call sequence again, only after the first finished.
    let second = cir_trace::spawn("second", helper);
    second.join().expect("helper thread panicked");

    println!("DONE done=1");
 cir_trace::finish();}
