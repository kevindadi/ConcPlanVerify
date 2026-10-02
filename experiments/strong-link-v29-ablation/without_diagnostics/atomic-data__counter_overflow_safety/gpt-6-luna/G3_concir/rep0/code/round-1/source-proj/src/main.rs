use std::sync::{Arc, Mutex};
use std::thread;

struct State {
    c: i32,
}

fn w1(m: Arc<Mutex<State>>) {
    let mut guard = m.lock().unwrap();
    if guard.c < 1 {
        guard.c = guard.c + 1;
    }
    drop(guard);
}

fn w2(m: Arc<Mutex<State>>) {
    let mut guard = m.lock().unwrap();
    if guard.c < 1 {
        guard.c = guard.c + 1;
    }
    drop(guard);
}

fn main() {
    let m = Arc::new(Mutex::new(State { c: 0 }));

    let m1 = Arc::clone(&m);
    let t1 = thread::spawn(move || w1(m1));

    let m2 = Arc::clone(&m);
    let t2 = thread::spawn(move || w2(m2));

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done=1");
}
