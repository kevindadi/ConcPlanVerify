mod cir_trace;
fn helper() {}

fn main() { cir_trace::init();
    let mut done: i32 = 0;
    helper();
    helper();
    done = 1;
    println!("DONE done={}", done);
 cir_trace::finish();}
