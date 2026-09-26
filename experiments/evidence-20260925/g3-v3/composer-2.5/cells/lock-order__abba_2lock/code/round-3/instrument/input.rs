use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

struct Main {
    a: Mutex<()>,
    b: Mutex<()>,
    t1: AtomicI32,
    t2: AtomicI32,
}

fn t1(m: Arc<Main>) {
    let ga = m.a.lock().unwrap();
    let gb = m.b.lock().unwrap();
    m.t1.store(1, Ordering::Relaxed);
    drop(gb);
    drop(ga);
}

fn t2(m: Arc<Main>) {
    let ga = m.a.lock().unwrap();
    let gb = m.b.lock().unwrap();
    m.t2.store(1, Ordering::Relaxed);
    drop(gb);
    drop(ga);
}

fn main() {
    let m = Arc::new(Main {
        a: Mutex::new(()),
        b: Mutex::new(()),
        t1: AtomicI32::new(0),
        t2: AtomicI32::new(0),
    });

    thread::scope(|s| {
        s.spawn(move || t1(Arc::clone(&m)));
        s.spawn(move || t2(Arc::clone(&m)));
    })
    .unwrap();

    println!(
        "DONE t1={} t2={}",
        m.t1.load(Ordering::Relaxed),
        m.t2.load(Ordering::Relaxed)
    );
}
