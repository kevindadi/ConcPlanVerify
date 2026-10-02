mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    let tmp = 1;
    drop(guard_b);
    drop(guard_a);
    let _ = tmp;
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    let tmp = 1;
    drop(guard_b);
    drop(guard_a);
    let _ = tmp;
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#477", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#515", ()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = cir_trace::spawn("t1#597", move || t1(a1, b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let h2 = cir_trace::spawn("t2#704", move || t2(a2, b2));

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE t1=1 t2=1");
 cir_trace::finish();}
