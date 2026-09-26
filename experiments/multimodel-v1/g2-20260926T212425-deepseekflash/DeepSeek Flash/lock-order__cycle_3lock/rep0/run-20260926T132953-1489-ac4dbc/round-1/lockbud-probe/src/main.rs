use std::sync::{Arc, Mutex};
use std::thread;
use concir_sync::Semaphore;

fn main() {
    // Three locks shared among the workers.
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));
    let c = Arc::new(Mutex::new(()));

    // A global ordering semaphore ensures no circular wait (deadlock-free).
    // Only one worker may attempt to acquire its locks at a time.
    let gate = Semaphore::new(1);

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let g1 = Arc::clone(&gate);
    let t1 = thread::spawn(move || {
        let _permit = g1.acquire();
        let _la = a1.lock().unwrap();
        let _lb = b1.lock().unwrap();
        // critical work
    });

    let b2 = Arc::clone(&b);
    let c2 = Arc::clone(&c);
    let g2 = Arc::clone(&gate);
    let t2 = thread::spawn(move || {
        let _permit = g2.acquire();
        let _lb = b2.lock().unwrap();
        let _lc = c2.lock().unwrap();
        // critical work
    });

    let c3 = Arc::clone(&c);
    let a3 = Arc::clone(&a);
    let g3 = Arc::clone(&gate);
    let t3 = thread::spawn(move || {
        let _permit = g3.acquire();
        let _lc = c3.lock().unwrap();
        let _la = a3.lock().unwrap();
        // critical work
    });

    t1.join().unwrap();
    t2.join().unwrap();
    t3.join().unwrap();

    println!("DONE done=1");
}
