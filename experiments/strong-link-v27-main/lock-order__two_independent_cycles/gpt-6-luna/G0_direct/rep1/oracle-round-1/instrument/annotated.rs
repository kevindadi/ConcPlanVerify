mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

type Lock = Arc<Mutex<()>>;

fn t1(a: Lock, b: Lock) {
    let first = a.lock().unwrap();
    let second = b.lock().unwrap();
    std::hint::black_box(());
    drop(second);
    drop(first);
}

fn t2(a: Lock, b: Lock) {
    let first = a.lock().unwrap();
    let second = b.lock().unwrap();
    std::hint::black_box(());
    drop(second);
    drop(first);
}

fn t3(c: Lock, d: Lock) {
    let first = c.lock().unwrap();
    let second = d.lock().unwrap();
    std::hint::black_box(());
    drop(second);
    drop(first);
}

fn t4(c: Lock, d: Lock) {
    let first = c.lock().unwrap();
    let second = d.lock().unwrap();
    std::hint::black_box(());
    drop(second);
    drop(first);
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#776", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#814", ()));
    let c = Arc::new(Mutex::new_named("c_mutex0#852", ()));
    let d = Arc::new(Mutex::new_named("d_mutex0#890", ()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = cir_trace::spawn("t1#972", move || t1(a1, b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let h2 = cir_trace::spawn("t2#1079", move || t2(a2, b2));

    let c3 = Arc::clone(&c);
    let d3 = Arc::clone(&d);
    let h3 = cir_trace::spawn("t3#1186", move || t3(c3, d3));

    let c4 = Arc::clone(&c);
    let d4 = Arc::clone(&d);
    let h4 = cir_trace::spawn("t4#1293", move || t4(c4, d4));

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();
    h4.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
