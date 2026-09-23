use std::sync::{Arc, Mutex};
use std::thread;

struct Locks {
    a: Mutex<()>,
    b: Mutex<()>,
}

fn w1(locks: Arc<Locks>) {
    let _ga = locks.a.lock().unwrap();
    let _gb = locks.b.lock().unwrap();
    drop(_gb);
    drop(_ga);
}

fn w2(locks: Arc<Locks>) {
    let _ga = locks.a.lock().unwrap();
    let _gb = locks.b.lock().unwrap();
    drop(_gb);
    drop(_ga);
}

fn main() {
    let locks = Arc::new(Locks {
        a: Mutex::new(()),
        b: Mutex::new(()),
    });

    let l1 = Arc::clone(&locks);
    let l2 = Arc::clone(&locks);

    let h1 = thread::spawn(move || w1(l1));
    let h2 = thread::spawn(move || w2(l2));

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
}
