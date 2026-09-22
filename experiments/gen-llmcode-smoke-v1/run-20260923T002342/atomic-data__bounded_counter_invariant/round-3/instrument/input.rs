use std::sync::{Arc, Mutex};
use std::thread;

struct Shared {
    c: i32,
    m: Mutex<()>,
    done: bool,
}

fn w2(s: Arc<Mutex<Shared>>) {
    let guard = s.lock().unwrap();
    let tmp = guard.c;
    let tmp2 = tmp + 1;
    // mutate through the guard
    let mut guard = guard;
    guard.c = tmp2;
    drop(guard);
}

fn w1(s: Arc<Mutex<Shared>>) {
    let h1 = thread::spawn({
        let s = Arc::clone(&s);
        move || w2(s)
    });
    let h2 = thread::spawn({
        let s = Arc::clone(&s);
        move || w2(s)
    });
    h1.join().unwrap();
    h2.join().unwrap();
    {
        let mut guard = s.lock().unwrap();
        guard.done = true;
    }
}

fn main() {
    let s = Arc::new(Mutex::new(Shared {
        c: 0,
        m: Mutex::new(()),
        done: false,
    }));

    let sup = thread::spawn({
        let s = Arc::clone(&s);
        move || w1(s)
    });
    sup.join().unwrap();

    let done = {
        let guard = s.lock().unwrap();
        guard.done
    };
    println!("DONE done={}", if done { 1 } else { 0 });
}
