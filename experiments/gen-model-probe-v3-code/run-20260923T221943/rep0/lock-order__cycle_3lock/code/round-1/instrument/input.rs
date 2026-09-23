use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));
    let c = Arc::new(Mutex::new(()));
    let mut done = 0;

    let t1 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        move || {
            let guard_a = a.lock().unwrap();
            let guard_b = b.lock().unwrap();
            drop(guard_b);
            drop(guard_a);
        }
    };

    let t2 = {
        let b = Arc::clone(&b);
        let c = Arc::clone(&c);
        move || {
            let guard_b = b.lock().unwrap();
            let guard_c = c.lock().unwrap();
            drop(guard_c);
            drop(guard_b);
        }
    };

    let t3 = {
        let a = Arc::clone(&a);
        let c = Arc::clone(&c);
        move || {
            let guard_a = a.lock().unwrap();
            let guard_c = c.lock().unwrap();
            drop(guard_c);
            drop(guard_a);
        }
    };

    let h1 = thread::spawn(t1);
    let h2 = thread::spawn(t2);
    let h3 = thread::spawn(t3);

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    done = 1;
    println!("DONE done={}", done);
}
