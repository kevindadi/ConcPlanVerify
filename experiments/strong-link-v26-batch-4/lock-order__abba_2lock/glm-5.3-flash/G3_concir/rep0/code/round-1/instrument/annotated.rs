mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};

fn t1(a: &Mutex<()>, b: &Mutex<()>) {
    // t1: lock a, then lock b, do critical work, unlock b, unlock a
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
    let work = 1;
    let _ = work;
    drop(_gb);
    drop(_ga);
}

fn t2(a: &Mutex<()>, b: &Mutex<()>) {
    // t2: lock a, then lock b, do critical work, unlock b, unlock a
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
    let work = 1;
    let _ = work;
    drop(_gb);
    drop(_ga);
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#554", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#592", ()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = cir_trace::spawn("t1#674", move || t1(&a1, &b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let h2 = cir_trace::spawn("t2#788", move || t2(&a2, &b2));

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE t1=1 t2=1");
 cir_trace::finish();}
