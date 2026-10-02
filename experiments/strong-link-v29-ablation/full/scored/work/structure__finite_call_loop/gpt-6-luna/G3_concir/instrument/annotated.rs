mod cir_trace;
fn helper() {}

fn main() { crate::cir_trace::init();
    helper();
    helper();
    println!("DONE done=1");
 crate::cir_trace::finish();}
