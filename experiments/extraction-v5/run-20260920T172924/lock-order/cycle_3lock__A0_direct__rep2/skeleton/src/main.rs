mod cir_trace;
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let mtx_a = Arc::new(Mutex::new(()));
    let mtx_b = Arc::new(Mutex::new(()));
    let mtx_c = Arc::new(Mutex::new(()));

    let (a1, b1) = (Arc::clone(&mtx_a), Arc::clone(&mtx_b));
    cir_trace::ev(&cir_trace::tag_str(), "L1"); let w1 = thread::spawn(move || {cir_trace::set_tag("tL1"); 
        cir_trace::ev(&cir_trace::tag_str(), "L2"); let ga = a1.lock().unwrap();
        cir_trace::ev(&cir_trace::tag_str(), "L3"); let gb = b1.lock().unwrap();
        drop(gb);
        drop(ga);
    });

    let (b2, c2) = (Arc::clone(&mtx_b), Arc::clone(&mtx_c));
    cir_trace::ev(&cir_trace::tag_str(), "L4"); let w2 = thread::spawn(move || {cir_trace::set_tag("tL4"); 
        cir_trace::ev(&cir_trace::tag_str(), "L5"); let gb = b2.lock().unwrap();
        cir_trace::ev(&cir_trace::tag_str(), "L6"); let gc = c2.lock().unwrap();
        drop(gc);
        drop(gb);
    });

    let (c3, a3) = (Arc::clone(&mtx_c), Arc::clone(&mtx_a));
    cir_trace::ev(&cir_trace::tag_str(), "L7"); let w3 = thread::spawn(move || {cir_trace::set_tag("tL7"); 
        cir_trace::ev(&cir_trace::tag_str(), "L8"); let gc = c3.lock().unwrap();
        cir_trace::ev(&cir_trace::tag_str(), "L9"); let ga = a3.lock().unwrap();
        drop(ga);
        drop(gc);
    });

    cir_trace::ev(&cir_trace::tag_str(), "L10"); w1.join().unwrap();
    cir_trace::ev(&cir_trace::tag_str(), "L11"); w2.join().unwrap();
    cir_trace::ev(&cir_trace::tag_str(), "L12"); w3.join().unwrap();

    println!("DONE done=1");
cir_trace::finish(); }
