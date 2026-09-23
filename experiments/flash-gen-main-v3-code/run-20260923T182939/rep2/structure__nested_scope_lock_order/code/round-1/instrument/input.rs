use std::sync::{Arc, Mutex};
use std::thread;

struct Locks {
    a: Mutex<()>,
    b: Mutex<()>,
}

fn x1(locks: Arc<Locks>) {
    {
        let _ga = locks.a.lock().unwrap();
        let _gb = locks.b.lock().unwrap();
    }
}

fn x2(locks: Arc<Locks>) {
    {
        let _ga = locks.a.lock().unwrap();
        let _gb = locks.b.lock().unwrap();
    }
}

fn outer(locks: Arc<Locks>) {
    let l1 = Arc::clone(&locks);
    let l2 = Arc::clone(&locks);
    let h1 = thread::spawn(move || x1(l1));
    let h2 = thread::spawn(move || x2(l2));
    h1.join().unwrap();
    h2.join().unwrap();
}

fn main() {
    let locks = Arc::new(Locks {
        a: Mutex::new(()),
        b: Mutex::new(()),
    });
    let l = Arc::clone(&locks);
    let h_outer = thread::spawn(move || outer(l));
    h_outer.join().unwrap();
    println!("DONE done=1");
}
