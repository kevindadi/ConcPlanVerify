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
    let t1 = thread::spawn(move || {
        // Acquire in a global order: a then b
        let _ga = l1.a.lock().unwrap();
        let _gb = l1.b.lock().unwrap();
        // critical work
        drop(_gb);
        drop(_ga);
    });

    let l2 = Arc::clone(&locks);
    let t2 = thread::spawn(move || {
        // Acquire in a global order: b then c
        let _gb = l2.b.lock().unwrap();
        let _gc = l2.c.lock().unwrap();
        // critical work
        drop(_gc);
        drop(_gb);
    });

    let l3 = Arc::clone(&locks);
    let t3 = thread::spawn(move || {
        // Acquire in a global order: c then a
        let _gc = l3.c.lock().unwrap();
        let _ga = l3.a.lock().unwrap();
        // critical work
        drop(_ga);
        drop(_gc);
    });

    t1.join().unwrap();
    t2.join().unwrap();
    t3.join().unwrap();

    println!("DONE done=1");
}
