mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
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
        cir_trace::spawn("s1", move || w2(shared))
    };
    let s2 = {
        let shared = Arc::clone(&shared);
        cir_trace::spawn("s2", move || w2(shared))
    };
    s1.join().unwrap();
    s2.join().unwrap();
    {
        let mut s = shared.lock().unwrap();
        s.done = true;
    }
}

fn main() { cir_trace::init();
    let shared = Arc::new(Mutex::new_named("shared_mutex0", Shared {
        c: 0,
        m: Mutex::new_named("shared_mutex1", ()),
        done: false,
    }));
    let sup = {
        let shared = Arc::clone(&shared);
        cir_trace::spawn("sup", move || w1(shared))
    };
    sup.join().unwrap();
    let done = {
        let s = shared.lock().unwrap();
        s.done
    };
    println!("DONE done={}", if done { 1 } else { 0 });
 cir_trace::finish();}
