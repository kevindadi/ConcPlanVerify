mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

#[allow(unused_imports)]
use concir_sync::Semaphore;

// Total lock order: a before b before c. Each worker acquires its two locks in
// that order, so the wait-for graph cannot contain a cycle.
fn t1(a: Arc<Mutex<i32>>, b: Arc<Mutex<i32>>) {
    let mut ga = a.lock().unwrap();
    let mut gb = b.lock().unwrap();
    *ga += 1;
    *gb += 1;
    drop(gb);
    drop(ga);
}

fn t2(b: Arc<Mutex<i32>>, c: Arc<Mutex<i32>>) {
    let mut gb = b.lock().unwrap();
    let mut gc = c.lock().unwrap();
    *gb += 1;
    *gc += 1;
    drop(gc);
    drop(gb);
}

fn t3(a: Arc<Mutex<i32>>, c: Arc<Mutex<i32>>) {
    let mut ga = a.lock().unwrap();
    let mut gc = c.lock().unwrap();
    *ga += 1;
    *gc += 1;
    drop(gc);
    drop(ga);
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0", 0));
    let b = Arc::new(Mutex::new_named("b_mutex0", 0));
    let c = Arc::new(Mutex::new_named("c_mutex0", 0));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = cir_trace::spawn("t1", move || t1(a1, b1));

    let b2 = Arc::clone(&b);
    let c2 = Arc::clone(&c);
    let h2 = cir_trace::spawn("t2", move || t2(b2, c2));

    let a3 = Arc::clone(&a);
    let c3 = Arc::clone(&c);
    let h3 = cir_trace::spawn("t3", move || t3(a3, c3));

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
