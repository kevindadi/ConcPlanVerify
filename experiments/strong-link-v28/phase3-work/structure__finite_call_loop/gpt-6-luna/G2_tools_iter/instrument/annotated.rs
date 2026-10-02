mod cir_trace;
fn helper() {}

fn main() { cir_trace::init();
    for _ in 0..2 {
        helper();
    }

    println!("DONE done=1");
 cir_trace::finish();}
