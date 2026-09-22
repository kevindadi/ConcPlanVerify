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
            let g = c1.lock().unwrap();
            *g
        };
        let tmp2 = tmp + 1;
        {
            let mut g = c1.lock().unwrap();
            *g = tmp2;
        }
    });

    let m2 = Arc::clone(&m);
    let c2 = Arc::clone(&c);
    let w2 = thread::spawn(move || {
        let _guard = m2.lock().unwrap();
        let tmp = {
            let g = c2.lock().unwrap();
            *g
        };
        let tmp2 = tmp + 1;
        {
            let mut g = c2.lock().unwrap();
            *g = tmp2;
        }
    });

    w1.join().unwrap();
    w2.join().unwrap();

    println!("DONE done=1");
}
