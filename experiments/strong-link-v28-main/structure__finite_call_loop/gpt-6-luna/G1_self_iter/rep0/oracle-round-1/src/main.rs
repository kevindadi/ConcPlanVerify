mod cir_trace;
fn helper() {}

fn main() { cir_trace::init();
    std::thread::spawn(helper).join().unwrap();
    std::thread::spawn(helper).join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
