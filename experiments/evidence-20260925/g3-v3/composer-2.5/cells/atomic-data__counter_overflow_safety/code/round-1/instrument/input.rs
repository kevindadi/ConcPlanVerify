use std::sync::{Arc, Mutex};
use std::thread;

struct Shared {
    c: i32,
}

fn w1(m: Arc<Mutex<Shared>>) {
    let mut guard = m.lock().unwrap();
    let tmp = guard.c;
    if tmp < 1 {
        guard.c = tmp + 1;
    }
}

fn w2(m: Arc<Mutex<Shared>>) {
    let mut guard = m.lock().unwrap();
    let tmp = guard.c;
    if tmp < 1 {
        guard.c = tmp + 1;
    }
}

fn main() {
    let m = Arc::new(Mutex::new(Shared { c: 0 }));

    let m_w1 = Arc::clone(&m);
    let h1 = thread::spawn(move || w1(m_w1));

    let m_w2 = Arc::clone(&m);
    let h2 = thread::spawn(move || w2(m_w2));

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
}
