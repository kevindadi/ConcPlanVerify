mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Arc};
use std::thread;

fn t1(a: &Mutex<()>, b: &Mutex<()>, t1: &AtomicI32) {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    t1.store(1, Ordering::SeqCst);
    drop(guard_b);
    drop(guard_a);
}

fn t2(a: &Mutex<()>, b: &Mutex<()>, t2: &AtomicI32) {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    t2.store(1, Ordering::SeqCst);
    drop(guard_b);
    drop(guard_a);
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0", ()));
    let t1 = Arc::new(AtomicI32::new(0));
    let t2 = Arc::new(AtomicI32::new(0));
    let done = Arc::new(AtomicBool::new(false));

    let h1 = cir_trace::spawn("h1", {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        let t1 = Arc::clone(&t1);
        move || crate::t1(&a, &b, &t1)
    });
    let h2 = cir_trace::spawn("h2", {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        let t2 = Arc::clone(&t2);
        move || crate::t2(&a, &b, &t2)
    });

    h1.join().unwrap();
    h2.join().unwrap();

    done.store(true, Ordering::SeqCst);

    println!(
        "DONE t1={} t2={}",
        t1.load(Ordering::SeqCst),
        t2.load(Ordering::SeqCst)
    );
 cir_trace::finish();}
