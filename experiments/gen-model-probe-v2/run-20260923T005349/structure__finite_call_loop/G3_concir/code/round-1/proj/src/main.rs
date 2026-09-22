mod cir_trace;
fn helper() {
    let x: i32 = 1;
    let _ = x;
}

fn main() { cir_trace::init();
    helper();
    helper();
    let done: i32 = 1;
    println!("DONE done={}", done);
 cir_trace::finish();}
