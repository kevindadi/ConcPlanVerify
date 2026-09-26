mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

use concir_sync::Semaphore;

fn w1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    drop(guard_b);
    drop(guard_a);
}

fn w2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    drop(guard_b);
    drop(guard_a);
}

fn main() { cir_trace::init();
    let _sem = Semaphore::new_named("_sem_semaphore0", 2);
    let a = Arc::new(Mutex::new_named("a_mutex0", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0", ()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let t1 = cir_trace::spawn("w1", move || w1(a1, b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let t2 = cir_trace::spawn("w2", move || w2(a2, b2));

    t1.join().unwrap();
    t2.join().unwrap();
    println!("DONE done=1");
 cir_trace::finish();}
