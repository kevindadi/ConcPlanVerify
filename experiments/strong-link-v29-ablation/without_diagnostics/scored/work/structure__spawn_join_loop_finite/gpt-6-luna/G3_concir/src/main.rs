mod cir_trace;
use std::thread;

fn worker() {}

fn main() { crate::cir_trace::init();
    let mut i = 0;

    while i < 2 {
        let worker_handle = crate::cir_trace::spawn("worker#112", move || worker());
        worker_handle.join().unwrap();
        i += 1;
    }

    println!("DONE done=1");
 crate::cir_trace::finish();}
