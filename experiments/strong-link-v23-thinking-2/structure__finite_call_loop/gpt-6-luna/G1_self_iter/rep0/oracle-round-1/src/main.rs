mod cir_trace;
fn helper() {}

fn main() { cir_trace::init();
    for _ in 0..2 {
        std::thread::spawn(helper)
            .join()
            .expect("helper thread panicked");
    }

    println!("DONE done=1");
 cir_trace::finish();}
