mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};

fn t1(a: &Arc<Mutex<()>>, b: &Arc<Mutex<()>>) {
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
    let work: i32 = 1;
    let _ = work;
    drop(_gb);
    drop(_ga);
}

fn t2(b: &Arc<Mutex<()>>, c: &Arc<Mutex<()>>) {
    let _gb = b.lock().unwrap();
    let _gc = c.lock().unwrap();
    let work: i32 = 2;
    let _ = work;
    drop(_gc);
    drop(_gb);
}

fn t3(a: &Arc<Mutex<()>>, c: &Arc<Mutex<()>>) {
    let _ga = a.lock().unwrap();
    let _gc = c.lock().unwrap();
    let work: i32 = 3;
    let _ = work;
    drop(_gc);
    drop(_ga);
}

fn main() { crate::cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#634", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#672", ()));
    let c = Arc::new(Mutex::new_named("c_mutex0#710", ()));

    let h1 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        crate::cir_trace::spawn("t1#808", move || t1(&a, &b))
    };
    let h2 = {
        let b = Arc::clone(&b);
        let c = Arc::clone(&c);
        crate::cir_trace::spawn("t2#941", move || t2(&b, &c))
    };
    let h3 = {
        let a = Arc::clone(&a);
        let c = Arc::clone(&c);
        crate::cir_trace::spawn("t3#1074", move || t3(&a, &c))
    };

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    let done: i32 = 1;
    println!("DONE done={}", done);
 crate::cir_trace::finish();}
