mod cir_trace;
// helper: a pure auxiliary routine with no shared state (R2).
// Each call runs to completion before the next call begins (R3);
// every schedule terminates trivially (R4).

fn helper(x: u64) -> u64 {
    x + 1
}

fn main() { cir_trace::init();
    // R1: main calls the auxiliary routine, then begins the same
    // call sequence again.
    let result = helper(0); // first run of the call sequence
    let _again = helper(0); // the same call sequence, started again

    // R5: print exactly `DONE done=1` then exit.
    println!("DONE done={}", result);
 cir_trace::finish();}
