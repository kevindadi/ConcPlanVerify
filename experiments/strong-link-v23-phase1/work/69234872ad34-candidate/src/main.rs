mod cir_trace;
fn helper() {}

fn call_sequence() {
    std::thread::spawn(helper)
        .join()
        .expect("helper thread panicked");
}

fn main() { cir_trace::init();
    call_sequence();
    call_sequence();
    println!("DONE done=1");
 cir_trace::finish();}
