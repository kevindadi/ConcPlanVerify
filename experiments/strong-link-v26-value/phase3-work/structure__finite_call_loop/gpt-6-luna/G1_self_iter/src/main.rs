mod cir_trace;
use std::thread;

fn helper() {}

fn main() { cir_trace::init();
    thread::spawn(helper).join().expect("helper panicked");
    thread::spawn(helper).join().expect("helper panicked");

    println!("DONE done=1");
 cir_trace::finish();}
