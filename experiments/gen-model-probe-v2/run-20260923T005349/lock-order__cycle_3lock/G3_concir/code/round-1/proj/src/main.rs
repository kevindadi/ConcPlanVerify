mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    drop(guard_b);
    drop(guard_a);
}

fn t2(b: Arc<Mutex<()>>, c: Arc<Mutex<()>>) {
    let guard_b = b.lock().unwrap();
    let guard_c = c.lock().unwrap();
    drop(guard_c);
    drop(guard_b);
}

fn t3(a: Arc<Mutex<()>>, c: Arc<Mutex<()>>) {
    let guard_a = a.lock().unwrap();
    let guard_c = c.lock().unwrap();
    drop(guard_c);
    drop(guard_a);
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0", ()));
    let c = Arc::new(Mutex::new_named("c_mutex0", ()));
    let mut done = 0;

    let h1 = cir_trace::spawn("h1", {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        move || t1(a, b)
    });
    let h2 = cir_trace::spawn("h2", {
        let b = Arc::clone(&b);
        let c = Arc::clone(&c);
        move || t2(b, c)
    });
    let h3 = cir_trace::spawn("h3", {
        let a = Arc::clone(&a);
        let c = Arc::clone(&c);
        move || t3(a, c)
    });

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    done = 1;
    println!("DONE done={}", done);
 cir_trace::finish();}
