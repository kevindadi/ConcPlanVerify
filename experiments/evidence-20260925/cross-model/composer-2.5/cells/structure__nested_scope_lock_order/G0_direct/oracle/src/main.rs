mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use concir_sync::Semaphore;
use std::sync::{Arc};
use std::thread;

fn x1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
}

fn x2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
}

fn outer(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = cir_trace::spawn("x1", move || x1(a1, b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let h2 = cir_trace::spawn("x2", move || x2(a2, b2));

    h1.join().unwrap();
    h2.join().unwrap();
}

fn main() { cir_trace::init();
    let _sem = Semaphore::new_named("_sem_semaphore0", 1);
    let a = Arc::new(Mutex::new_named("a_mutex0", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0", ()));

    let a_outer = Arc::clone(&a);
    let b_outer = Arc::clone(&b);
    let outer_handle = cir_trace::spawn("outer", move || outer(a_outer, b_outer));

    outer_handle.join().unwrap();
    println!("DONE done=1");
 cir_trace::finish();}
