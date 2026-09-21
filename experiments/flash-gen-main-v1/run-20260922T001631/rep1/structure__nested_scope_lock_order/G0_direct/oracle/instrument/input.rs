use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    let outer = thread::spawn({
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        move || {
            let inner1 = thread::spawn({
                let a = Arc::clone(&a);
                let b = Arc::clone(&b);
                move || {
                    let _ga = a.lock().unwrap();
                    let _gb = b.lock().unwrap();
                }
            });

            let inner2 = thread::spawn({
                let a = Arc::clone(&a);
                let b = Arc::clone(&b);
                move || {
                    let _ga = a.lock().unwrap();
                    let _gb = b.lock().unwrap();
                }
            });

            inner1.join().unwrap();
            inner2.join().unwrap();
        }
    });

    outer.join().unwrap();

    println!("DONE done=1");
}
