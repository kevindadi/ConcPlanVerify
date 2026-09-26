mod cir_trace;
fn helper() {
}

fn main() { cir_trace::init();
    helper();
    helper();
    println!("DONE done=1");
 cir_trace::finish();}

// kept-comment
