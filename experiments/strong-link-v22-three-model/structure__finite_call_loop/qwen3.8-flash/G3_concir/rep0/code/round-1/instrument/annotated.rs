mod cir_trace;
use std::sync::Arc;

fn helper() {
    // no operations
}

fn main() { cir_trace::init();
    let mut done = 0;

    helper();
    done = 1;

    println!("DONE done={}", done);
 cir_trace::finish();}
