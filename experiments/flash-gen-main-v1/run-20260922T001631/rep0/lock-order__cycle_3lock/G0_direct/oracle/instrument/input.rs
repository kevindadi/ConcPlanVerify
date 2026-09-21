use std::sync::{Arc, Mutex};
use std::thread;

struct Locks {
    a: Mutex<()>,
    b: Mutex<()>,
    c: Mutex<()>,
}

fn main() {
    let locks = Arc::new(Locks {
        a: Mutex::new(()),
        b: Mutex::new(()),
        c: Mutex::new(()),
    });

    let l1 = Arc::clone(&locks);
    let l2 = Arc::clone(&locks);
    let l3 = Arc::clone(&locks);

    // Worker 1 needs locks a and b.
    let w1 = thread::spawn(move || {
        let _ga = l1.a.lock().unwrap();
        let _gb = l1.b.lock().unwrap();
        // critical work
    });

    // Worker 2 needs locks b and c.
    let w2 = thread::spawn(move || {
        let _gb = l2.b.lock().unwrap();
        let _gc = l2.c.lock().unwrap();
        // critical work
    });

    // Worker 3 needs locks c and a.
    let w3 = thread::spawn(move || {
        let _gc = l3.c.lock().unwrap();
        let _ga = l3.a.lock().unwrap();
        // critical work
    });

    w1.join().unwrap();
    w2.join().unwrap();
    w3.join().unwrap();

    println!("DONE done=1");
}
