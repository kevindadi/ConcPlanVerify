mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();

    // Work while holding both locks.
    std::hint::black_box(());

    drop(b_guard);
    drop(a_guard);
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();

    // Work while holding both locks.
    std::hint::black_box(());

    drop(b_guard);
    drop(a_guard);
}

fn t3(c: Arc<Mutex<()>>, d: Arc<Mutex<()>>) {
    let c_guard = c.lock().unwrap();
    let d_guard = d.lock().unwrap();

    // Work while holding both locks.
    std::hint::black_box(());

    drop(d_guard);
    drop(c_guard);
}

fn t4(c: Arc<Mutex<()>>, d: Arc<Mutex<()>>) {
    let c_guard = c.lock().unwrap();
    let d_guard = d.lock().unwrap();

    // Work while holding both locks.
    std::hint::black_box(());

    drop(d_guard);
    drop(c_guard);
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#1011", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#1049", ()));
    let c = Arc::new(Mutex::new_named("c_mutex0#1087", ()));
    let d = Arc::new(Mutex::new_named("d_mutex0#1125", ()));

    let h1 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        cir_trace::spawn("t1#1223", move || t1(a, b))
    };
    let h2 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        cir_trace::spawn("t2#1349", move || t2(a, b))
    };
    let h3 = {
        let c = Arc::clone(&c);
        let d = Arc::clone(&d);
        cir_trace::spawn("t3#1475", move || t3(c, d))
    };
    let h4 = {
        let c = Arc::clone(&c);
        let d = Arc::clone(&d);
        cir_trace::spawn("t4#1601", move || t4(c, d))
    };

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();
    h4.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
