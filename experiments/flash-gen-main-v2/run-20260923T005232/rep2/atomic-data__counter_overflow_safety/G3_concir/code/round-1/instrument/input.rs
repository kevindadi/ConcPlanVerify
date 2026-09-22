use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let m = Arc::new(Mutex::new(()));
    let c = Arc::new(Mutex::new(0i32));

    let m1 = Arc::clone(&m);
    let c1 = Arc::clone(&c);
    let w1 = thread::spawn(move || {
        let _guard = m1.lock().unwrap();
        let tmp = {
            let cg = c1.lock().unwrap();
            *cg
        };
        if tmp < 1 {
            let mut cg = c1.lock().unwrap();
            *cg = tmp + 1;
        }
    });

    let m2 = Arc::clone(&m);
    let c2 = Arc::clone(&c);
    let w2 = thread::spawn(move || {
        let _guard = m2.lock().unwrap();
        let tmp = {
            let cg = c2.lock().unwrap();
            *cg
        };
        if tmp < 1 {
            let mut cg = c2.lock().unwrap();
            *cg = tmp + 1;
        }
    });

    w1.join().unwrap();
    w2.join().unwrap();

    let done = {
        let cg = c.lock().unwrap();
        *cg
    };
    println!("DONE done={}", done);
}
