mod cir_trace;
fn helper() -> i32 {
    let mut done = 0;
    done = 1;
    done
}

fn main() { cir_trace::init();
    let done = helper();
    let _ = helper();
    println!("DONE done={}", done);
 cir_trace::finish();}
