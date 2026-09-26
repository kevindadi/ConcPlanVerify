mod cir_trace;
fn helper() {
    let mut i: i32 = 0;
    while i < 2 {
        i += 1;
    }
}

fn main() { cir_trace::init();
    helper();
    helper();
    let done: i32 = 1;
    println!("DONE done={}", done);
 cir_trace::finish();}
