use std::sync::{Arc, Mutex};
use std::thread;

struct Shared {
    c: i32,
    done: bool,
}

fn w2(shared: Arc<Mutex<Shared>>) {
    let mut guard = shared.lock().unwrap();
    let cur = guard.c;
    if cur < 1 {
        guard.c = cur + 1;
    }
    drop(guard);
}

fn w1(shared: Arc<Mutex<Shared>>) {
    let s1 = Arc::clone(&shared);
    let h1 = thread::spawn(move || w2(s1));
    let s2 = Arc::clone(&shared);
    let h2 = thread::spawn(move || w2(s2));
    h1.join().unwrap();
    h2.join().unwrap();
    let mut guard = shared.lock().unwrap();
    guard.done = true;
    drop(guard);
}

fn main() {
    let shared = Arc::new(Mutex::new(Shared { c: 0, done: false }));
    let s = Arc::clone(&shared);
    let sup = thread::spawn(move || w1(s));
    sup.join().unwrap();
    let guard = shared.lock().unwrap();
    let done = guard.done;
    drop(guard);
    println!("DONE done={}", if done { 1 } else { 0 });
}
