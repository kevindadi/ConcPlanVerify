mod cir_trace;
fn helper() {
    let mut work: i64 = 0;
    work = 1;
}

fn main() { cir_trace::init();
    helper();
    helper();
    println!("DONE done=1");
 cir_trace::finish();}
