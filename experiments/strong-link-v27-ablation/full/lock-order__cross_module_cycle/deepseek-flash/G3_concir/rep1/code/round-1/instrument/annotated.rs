mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let mut x = 0;
    let _guard_a = a.lock().unwrap();
    let _guard_b = b.lock().unwrap();
    x = 1;
    drop(_guard_b);
    drop(_guard_a);
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let mut x = 0;
    let _guard_a = a.lock().unwrap();
    let _guard_b = b.lock().unwrap();
    x = 2;
    drop(_guard_b);
    drop(_guard_a);
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

    println!("DONE done=1");
 cir_trace::finish();}
