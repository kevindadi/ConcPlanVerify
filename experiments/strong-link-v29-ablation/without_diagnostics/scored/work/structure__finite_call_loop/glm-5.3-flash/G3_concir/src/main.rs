mod cir_trace;
fn helper() {
    // auxiliary routine: runs to completion, no shared state
}

fn main() { crate::cir_trace::init();
    // R1: main calls the auxiliary routine, then begins the same call again
    helper();
    helper();

    // R5: print the exact terminal line
    println!("DONE done=1");
 crate::cir_trace::finish();}
