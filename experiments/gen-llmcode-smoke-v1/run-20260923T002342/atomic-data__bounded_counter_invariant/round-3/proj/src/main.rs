mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
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
    let h1 = cir_trace::spawn("h1", {
        let s = Arc::clone(&s);
        move || w2(s)
    });
    let h2 = cir_trace::spawn("h2", {
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

fn main() { cir_trace::init();
    let s = Arc::new(Mutex::new_named("s_mutex0", Shared {
        c: 0,
        m: Mutex::new_named("s_mutex1", ()),
        done: false,
    }));

    let sup = cir_trace::spawn("sup", {
        let s = Arc::clone(&s);
        move || w1(s)
    });
    sup.join().unwrap();

    let done = {
        let guard = s.lock().unwrap();
        guard.done
    };
    println!("DONE done={}", if done { 1 } else { 0 });
 cir_trace::finish();}
