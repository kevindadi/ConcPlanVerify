mod cir_trace;
fn aux() {
}

fn main() { cir_trace::init();
    aux();
    aux();
    println!("DONE done=1");
 cir_trace::finish();}
