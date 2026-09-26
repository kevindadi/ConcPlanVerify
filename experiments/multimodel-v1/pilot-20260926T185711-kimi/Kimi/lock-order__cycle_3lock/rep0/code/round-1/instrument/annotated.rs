mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) -> i32 {
    let mut work = 0;
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    work = 1;
    drop(guard_b);
    drop(guard_a);
    work
}

fn t2(b: Arc<Mutex<()>>, c: Arc<Mutex<()>>) -> i32 {
    let mut work = 0;
    let guard_b = b.lock().unwrap();
    let guard_c = c.lock().unwrap();
    work = 1;
    drop(guard_c);
    drop(guard_b);
    work
}

fn t3(a: Arc<Mutex<()>>, c: Arc<Mutex<()>>) -> i32 {
    let mut work = 0;
    let guard_a = a.lock().unwrap();
    let guard_c = c.lock().unwrap();
    work = 1;
    drop(guard_c);
    drop(guard_a);
    work
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0", ()));
    let c = Arc::new(Mutex::new_named("c_mutex0", ()));

    let h1 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        cir_trace::spawn("t1", move || t1(a, b))
    };
    let h2 = {
        let b = Arc::clone(&b);
        let c = Arc::clone(&c);
        cir_trace::spawn("t2", move || t2(b, c))
    };
    let h3 = {
        let a = Arc::clone(&a);
        let c = Arc::clone(&c);
        cir_trace::spawn("t3", move || t3(a, c))
    };

    let r1 = h1.join().unwrap();
    let r2 = h2.join().unwrap();
    let r3 = h3.join().unwrap();

    let done = if r1 == 1 && r2 == 1 && r3 == 1 { 1 } else { 0 };
    println!("DONE done={}", done);
 cir_trace::finish();}
