mod cir_trace;
fn worker() {
    // no operations
}

fn main() { cir_trace::init();
    let mut i = 0;
    while i < 2 {
        let h = cir_trace::spawn("h", worker);
        h.join().unwrap();
        i += 1;
    }
    println!("DONE done=1");
 cir_trace::finish();}
