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

    // Global ordering to prevent deadlock (R5, R8, R9):
    // acquire locks in a consistent global order: a < b < c.
    // t1 needs a and b -> acquire a then b
    // t2 needs b and c -> acquire b then c
    // t3 needs c and a -> acquire a then c (a < c)

    let l1 = Arc::clone(&locks);
    let t1 = thread::spawn(move || {
        let _ga = l1.a.lock().unwrap();
        let _gb = l1.b.lock().unwrap();
        // critical work with both a and b held
        // locks released on drop (R6)
    });

    let l2 = Arc::clone(&locks);
    let t2 = thread::spawn(move || {
        let _gb = l2.b.lock().unwrap();
        let _gc = l2.c.lock().unwrap();
        // critical work with both b and c held
    });

    let l3 = Arc::clone(&locks);
    let t3 = thread::spawn(move || {
        let _ga = l3.a.lock().unwrap();
        let _gc = l3.c.lock().unwrap();
        // critical work with both a and c held
    });

    t1.join().unwrap();
    t2.join().unwrap();
    t3.join().unwrap();

    println!("DONE done=1");
}
