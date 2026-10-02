use std::sync::{Arc, Mutex};
use std::thread;

struct Counter {
    c: i32,
}

fn w1(m: Arc<Mutex<Counter>>) {
    let mut guard = m.lock().unwrap();
    let c = guard.c;
    guard.c = c + 1;
}

fn w2(m: Arc<Mutex<Counter>>) {
    let mut guard = m.lock().unwrap();
    let c = guard.c;
    guard.c = c + 1;
}

fn main() {
    let m = Arc::new(Mutex::new(Counter { c: 0 }));

    let t1 = thread::spawn({
        let m = Arc::clone(&m);
        move || w1(m)
    });
    let t2 = thread::spawn({
        let m = Arc::clone(&m);
        move || w2(m)
    });

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done=1");
}
