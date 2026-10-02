mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    let mut work = 0;
    work = 1;
    let _ = work;
    drop(guard_b);
    drop(guard_a);
}

fn t2(b: Arc<Mutex<()>>, c: Arc<Mutex<()>>) {
    let guard_b = b.lock().unwrap();
    let guard_c = c.lock().unwrap();
    let mut work = 0;
    work = 1;
    let _ = work;
    drop(guard_c);
    drop(guard_b);
}

fn t3(a: Arc<Mutex<()>>, c: Arc<Mutex<()>>) {
    let guard_a = a.lock().unwrap();
    let guard_c = c.lock().unwrap();
    let mut work = 0;
    work = 1;
    let _ = work;
    drop(guard_c);
    drop(guard_a);
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#732", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#770", ()));
    let c = Arc::new(Mutex::new_named("c_mutex0#808", ()));

    let h1 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        cir_trace::spawn("t1#906", move || t1(a, b))
    };
    let h2 = {
        let b = Arc::clone(&b);
        let c = Arc::clone(&c);
        cir_trace::spawn("t2#1032", move || t2(b, c))
    };
    let h3 = {
        let a = Arc::clone(&a);
        let c = Arc::clone(&c);
        cir_trace::spawn("t3#1158", move || t3(a, c))
    };

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
