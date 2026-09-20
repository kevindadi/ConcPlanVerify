mod cir_trace;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

fn main() {
    let (tx, rx) = mpsc::sync_channel::<i32>(1);
    let m = Arc::new(Mutex::new(()));

    let m1 = Arc::clone(&m);
    cir_trace::ev(&cir_trace::tag_str(), "L1"); let s = thread::spawn(move || {cir_trace::set_tag("tL1"); 
        // Send first value without holding the lock.
        cir_trace::ev(&cir_trace::tag_str(), "L2"); tx.send(1).unwrap();
        // Briefly take the lock, then release it before the second send.
        {
            cir_trace::ev(&cir_trace::tag_str(), "L3"); let _g = m1.lock().unwrap();
        }
        cir_trace::ev(&cir_trace::tag_str(), "L4"); tx.send(2).unwrap();
    });

    let m2 = Arc::clone(&m);
    cir_trace::ev(&cir_trace::tag_str(), "L5"); let r = thread::spawn(move || {cir_trace::set_tag("tL5"); 
        // Receive first value without holding the lock.
        cir_trace::ev(&cir_trace::tag_str(), "L6"); rx.recv().unwrap();
        // Briefly take the lock, then release it before the second recv.
        {
            cir_trace::ev(&cir_trace::tag_str(), "L7"); let _g = m2.lock().unwrap();
        }
        cir_trace::ev(&cir_trace::tag_str(), "L8"); rx.recv().unwrap();
    });

    cir_trace::ev(&cir_trace::tag_str(), "L9"); s.join().unwrap();
    cir_trace::ev(&cir_trace::tag_str(), "L10"); r.join().unwrap();
    println!("DONE done=1");
cir_trace::finish(); }
