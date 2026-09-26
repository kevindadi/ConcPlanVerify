mod cir_trace;
fn worker() {}

fn main() { cir_trace::init();
    let h1 = cir_trace::spawn("h1", worker);
    h1.join().unwrap();

    let h2 = cir_trace::spawn("h2", worker);
    h2.join().unwrap();

    let done = 1;
    println!("DONE done={}", done);
 cir_trace::finish();}
