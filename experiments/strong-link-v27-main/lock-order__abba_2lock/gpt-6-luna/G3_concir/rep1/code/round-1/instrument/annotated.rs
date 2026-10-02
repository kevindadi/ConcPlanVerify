mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();

    let mut critical_work = 0;
    critical_work = 1;
    let _ = critical_work;

    drop(guard_b);
    drop(guard_a);
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();

    let mut critical_work = 0;
    critical_work = 1;
    let _ = critical_work;

    drop(guard_b);
    drop(guard_a);
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#575", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#613", ()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let handle1 = cir_trace::spawn("t1#700", move || t1(a1, b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let handle2 = cir_trace::spawn("t2#812", move || t2(a2, b2));

    handle1.join().unwrap();
    handle2.join().unwrap();

    println!("DONE t1=1 t2=1");
 cir_trace::finish();}
