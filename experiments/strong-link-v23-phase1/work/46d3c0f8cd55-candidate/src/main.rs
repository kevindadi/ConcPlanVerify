mod cir_trace;
// Roles: helper (auxiliary routine), called by the main task.

fn helper(n: u32) {
    // No shared state: nothing here is shared with the main task,
    // so no two tasks ever contend for a resource. (R2)
    let _ = n;
}

fn main() { cir_trace::init();
    // R1: main task calls the auxiliary routine, then begins the
    // same call sequence again.
    // R3: each call runs to completion before the next call starts
    // (calls are strictly sequential on the single task).
    // R4: a single task with a fixed, finite call sequence always
    // terminates under every schedule.
    helper(1);
    helper(1);

    // R5: print exactly this line and exit.
    println!("DONE done=1");
 cir_trace::finish();}
