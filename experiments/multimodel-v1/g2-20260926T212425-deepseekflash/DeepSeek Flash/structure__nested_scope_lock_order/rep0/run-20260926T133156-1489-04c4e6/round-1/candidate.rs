use std::sync::{Arc, Mutex};
use std::thread;
use concir_sync::Semaphore;

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    let done = Arc::new(Semaphore::new(0));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let done1 = Arc::clone(&done);

    let outer = thread::spawn(move || {
        let x1 = thread::spawn(move || {
            let _ga = a1.lock().unwrap();
            let _gb = b1.lock().unwrap();
            done1.acquire();
        });

        let a2 = Arc::clone(&a);
        let b2 = Arc::clone(&b);
        let done2 = Arc::clone(&done);

        let x2 = thread::spawn(move || {
            let _ga = a2.lock().unwrap();
            let _gb = b2.lock().unwrap();
            done2.acquire();
        });

        x1.join().unwrap();
        x2.join().unwrap();
    });

    outer.join().unwrap();

    println!("DONE done=1");
}
