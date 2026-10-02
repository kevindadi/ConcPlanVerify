mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn work() {}

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _a = a.lock().unwrap();
    let _b = b.lock().unwrap();
    work();
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _a = a.lock().unwrap();
    let _b = b.lock().unwrap();
    work();
}

fn t3(c: Arc<Mutex<()>>, d: Arc<Mutex<()>>) {
    let _c = c.lock().unwrap();
    let _d = d.lock().unwrap();
    work();
}

fn t4(c: Arc<Mutex<()>>, d: Arc<Mutex<()>>) {
    let _c = c.lock().unwrap();
    let _d = d.lock().unwrap();
    work();
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#601", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#639", ()));
    let c = Arc::new(Mutex::new_named("c_mutex0#677", ()));
    let d = Arc::new(Mutex::new_named("d_mutex0#715", ()));

    let handles = [
        cir_trace::spawn("t1#754", {
            let a = Arc::clone(&a);
            let b = Arc::clone(&b);
            move || t1(a, b)
        }),
        cir_trace::spawn("t2#891", {
            let a = Arc::clone(&a);
            let b = Arc::clone(&b);
            move || t2(a, b)
        }),
        cir_trace::spawn("t3#1028", {
            let c = Arc::clone(&c);
            let d = Arc::clone(&d);
            move || t3(c, d)
        }),
        cir_trace::spawn("t4#1165", {
            let c = Arc::clone(&c);
            let d = Arc::clone(&d);
            move || t4(c, d)
        }),
    ];

    for handle in handles {
        handle.join().unwrap();
    }

    println!("DONE done=1");
 cir_trace::finish();}
