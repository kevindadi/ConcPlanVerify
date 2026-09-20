mod cir_trace;
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    let (a1, b1) = (Arc::clone(&a), Arc::clone(&b));
    cir_trace::ev(&cir_trace::tag_str(), "L1"); let x1 = thread::spawn(move || {cir_trace::set_tag("tL1"); 
        cir_trace::ev(&cir_trace::tag_str(), "L2"); let _ga = a1.lock().unwrap();
        cir_trace::ev(&cir_trace::tag_str(), "L3"); let _gb = b1.lock().unwrap();
    });

    let (a2, b2) = (Arc::clone(&a), Arc::clone(&b));
    cir_trace::ev(&cir_trace::tag_str(), "L4"); let x2 = thread::spawn(move || {cir_trace::set_tag("tL4"); 
        cir_trace::ev(&cir_trace::tag_str(), "L5"); let _ga = a2.lock().unwrap();
        cir_trace::ev(&cir_trace::tag_str(), "L6"); let _gb = b2.lock().unwrap();
    });

    cir_trace::ev(&cir_trace::tag_str(), "L7"); x1.join().unwrap();
    cir_trace::ev(&cir_trace::tag_str(), "L8"); x2.join().unwrap();
    println!("DONE done=1");
cir_trace::finish(); }
