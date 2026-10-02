use std::sync::{Arc, Mutex};
use std::thread;

struct Counter {
    c: i32,
}

fn w1(m: &Arc<Mutex<Counter>>) {
    let mut guard = m.lock().unwrap();
    let c = guard.c;
    if c < 1 {
        guard.c = c + 1;
    }
    drop(guard);
}

fn w2(m: &Arc<Mutex<Counter>>) {
    let mut guard = m.lock().unwrap();
    let c = guard.c;
    if c < 1 {
        guard.c = c + 1;
    }
    drop(guard);
}

fn main() {
    let m = Arc::new(Mutex::new(Counter { c: 0 }));

    let m1 = Arc::clone(&m);
    let t1 = thread::spawn(move || w1(&m1));
    let m2 = Arc::clone(&m);
    let t2 = thread::spawn(move || w2(&m2));

    t1.join().unwrap();
    t2.join().unwrap();

    let final_c = m.lock().unwrap().c;
    println!("DONE done={}", final_c);
}
