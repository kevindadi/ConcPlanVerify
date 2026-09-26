mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use concir_sync::Semaphore;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
}

fn t3(c: Arc<Mutex<()>>, d: Arc<Mutex<()>>) {
    let _gc = c.lock().unwrap();
    let _gd = d.lock().unwrap();
}

fn t4(c: Arc<Mutex<()>>, d: Arc<Mutex<()>>) {
    let _gc = c.lock().unwrap();
    let _gd = d.lock().unwrap();
}

fn main() { cir_trace::init();
    let _semaphore = Semaphore::new_named("_semaphore_semaphore0", 4);

    let a = Arc::new(Mutex::new_named("a_mutex0", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0", ()));
    let c = Arc::new(Mutex::new_named("c_mutex0", ()));
    let d = Arc::new(Mutex::new_named("d_mutex0", ()));
    let done = Arc::new(AtomicUsize::new(0));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = cir_trace::spawn("t1", move || t1(a1, b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let h2 = cir_trace::spawn("t2", move || t2(a2, b2));

    let c3 = Arc::clone(&c);
    let d3 = Arc::clone(&d);
    let h3 = cir_trace::spawn("t3", move || t3(c3, d3));

    let c4 = Arc::clone(&c);
    let d4 = Arc::clone(&d);
    let h4 = cir_trace::spawn("t4", move || t4(c4, d4));

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();
    h4.join().unwrap();

    done.store(1, Ordering::SeqCst);
    println!("DONE done={}", done.load(Ordering::SeqCst));
 cir_trace::finish();}
