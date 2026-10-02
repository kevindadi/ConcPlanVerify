mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn critical_work(a: &Mutex<()>, b: &Mutex<()>) -> usize {
    let _guard_a = a.lock().unwrap();
    let _guard_b = b.lock().unwrap();

    // Critical work while holding both locks.
    1
}

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) -> usize {
    critical_work(&a, &b)
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) -> usize {
    critical_work(&a, &b)
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#446", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#484", ()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = cir_trace::spawn("t1#566", move || t1(a1, b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let h2 = cir_trace::spawn("t2#673", move || t2(a2, b2));

    let result1 = h1.join().unwrap();
    let result2 = h2.join().unwrap();

    println!("DONE t1={} t2={}", result1, result2);
 cir_trace::finish();}
