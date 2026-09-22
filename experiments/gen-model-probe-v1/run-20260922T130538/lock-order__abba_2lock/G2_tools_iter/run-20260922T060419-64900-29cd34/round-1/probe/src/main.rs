use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let t1 = Arc::new(Mutex::new(0));
    let t2 = Arc::new(Mutex::new(0));

    let t1_for_w1 = Arc::clone(&t1);
    let t2_for_w1 = Arc::clone(&t2);
    let w1 = thread::spawn(move || {
        let mut g1 = t1_for_w1.lock().unwrap();
        let _g2 = t2_for_w1.lock().unwrap();
        *g1 = 1;
    });

    let t1_for_w2 = Arc::clone(&t1);
    let t2_for_w2 = Arc::clone(&t2);
    let w2 = thread::spawn(move || {
        let _g1 = t1_for_w2.lock().unwrap();
        let mut g2 = t2_for_w2.lock().unwrap();
        *g2 = 1;
    });

    w1.join().unwrap();
    w2.join().unwrap();

    let v1 = *t1.lock().unwrap();
    let v2 = *t2.lock().unwrap();
    println!("DONE t1={} t2={}", v1, v2);
}
