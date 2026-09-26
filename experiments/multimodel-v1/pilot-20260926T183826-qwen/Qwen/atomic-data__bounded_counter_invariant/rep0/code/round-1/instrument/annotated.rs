mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", 0));

    let m1 = Arc::clone(&m);
    let w1_handle = cir_trace::spawn("w1_handle", move || {
        let mut guard = m1.lock().unwrap();
        *guard += 1;
    });

    let m2 = Arc::clone(&m);
    let w2_handle = cir_trace::spawn("w2_handle", move || {
        let mut guard = m2.lock().unwrap();
        *guard += 1;
    });

    w1_handle.join().unwrap();
    w2_handle.join().unwrap();

    // After joins, read shared state to print the required terminal line.
    // The requirement says "DONE done=1". This implies that after both workers
    // increment the counter (which starts at 0 and goes to 2), we print a specific string.
    // Wait, R7 says: Print exactly the line `DONE done=1`.
    // It does not say `done=<value>`. It says literally `DONE done=1`.
    // Let's re-read carefully.
    // R7. The program must print exactly the line `DONE done=1` and then exit. [U]
    
    println!("DONE done=1");
 cir_trace::finish();}
