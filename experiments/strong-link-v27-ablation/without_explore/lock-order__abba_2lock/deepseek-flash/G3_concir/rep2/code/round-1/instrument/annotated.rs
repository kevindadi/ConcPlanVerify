mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#87", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#125", ()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = cir_trace::spawn("t1#207", move || t1(a1, b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let h2 = cir_trace::spawn("t2#314", move || t2(a2, b2));

    let r1 = h1.join().unwrap();
    let r2 = h2.join().unwrap();

    println!("DONE t1={} t2={}", r1, r2);
 cir_trace::finish();}

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) -> i32 {
    let ga = a.lock().unwrap();
    let gb = b.lock().unwrap();

    let tmp: i32 = 1;

    drop(gb);
    drop(ga);

    tmp
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) -> i32 {
    let ga = a.lock().unwrap();
    let gb = b.lock().unwrap();

    let tmp: i32 = 1;

    drop(gb);
    drop(ga);

    tmp
}
