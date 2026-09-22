use std::sync::{Arc, Mutex};
use std::thread;

struct Shared {
    c: i32,
    m: Mutex<()>,
    done: bool,
}

fn w2(shared: Arc<Mutex<Shared>>) {
    let mut s = shared.lock().unwrap();
    let _g = s.m.lock().unwrap();
    let tmp = s.c;
    let tmp2 = tmp + 1;
    s.c = tmp2;
    drop(_g);
    drop(s);
}

fn w1(shared: Arc<Mutex<Shared>>) {
    let s1 = {
        let shared = Arc::clone(&shared);
        thread::spawn(move || w2(shared))
    };
    let s2 = {
        let shared = Arc::clone(&shared);
        thread::spawn(move || w2(shared))
    };
    s1.join().unwrap();
    s2.join().unwrap();
    {
        let mut s = shared.lock().unwrap();
        s.done = true;
    }
}

fn main() {
    let shared = Arc::new(Mutex::new(Shared {
        c: 0,
        m: Mutex::new(()),
        done: false,
    }));
    let sup = {
        let shared = Arc::clone(&shared);
        thread::spawn(move || w1(shared))
    };
    sup.join().unwrap();
    let done = {
        let s = shared.lock().unwrap();
        s.done
    };
    println!("DONE done={}", if done { 1 } else { 0 });
}
