use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    let a_outer = Arc::clone(&a);
    let b_outer = Arc::clone(&b);

    let outer = thread::spawn(move || {
        let a1 = Arc::clone(&a_outer);
        let b1 = Arc::clone(&b_outer);
        let x1 = thread::spawn(move || {
            let _ga = a1.lock().unwrap();
            let _gb = b1.lock().unwrap();
        });

        let a2 = Arc::clone(&a_outer);
        let b2 = Arc::clone(&b_outer);
        let x2 = thread::spawn(move || {
            let _ga = a2.lock().unwrap();
            let _gb = b2.lock().unwrap();
        });

        x1.join().unwrap();
        x2.join().unwrap();
    });

    outer.join().unwrap();

    println!("DONE done=1");
}
