mod cir_trace;
fn helper() {}

fn main() { cir_trace::init();
    let handle = cir_trace::spawn("handle#45", helper);
    handle.join().unwrap();

    let handle = cir_trace::spawn("handle#119", helper);
    handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
