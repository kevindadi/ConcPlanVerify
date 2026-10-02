mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

// Worker t1: holds a then b at the same time.
fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let ga = a.lock().unwrap();
    let gb = b.lock().unwrap();
    let _x: i32 = 1;
    drop(gb);
    drop(ga);
}

// Worker t2: holds b then c at the same time.
fn t2(b: Arc<Mutex<()>>, c: Arc<Mutex<()>>) {
    let gb = b.lock().unwrap();
    let gc = c.lock().unwrap();
    let _x: i32 = 1;
    drop(gc);
    drop(gb);
}

// Worker t3: holds a then c at the same time.
fn t3(a: Arc<Mutex<()>>, c: Arc<Mutex<()>>) {
    let ga = a.lock().unwrap();
    let gc = c.lock().unwrap();
    let _x: i32 = 1;
    drop(gc);
    drop(ga);
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#714", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#752", ()));
    let c = Arc::new(Mutex::new_named("c_mutex0#790", ()));

    let h1 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        cir_trace::spawn("t1#888", move || t1(a, b))
    };

    let h2 = {
        let b = Arc::clone(&b);
        let c = Arc::clone(&c);
        cir_trace::spawn("t2#1015", move || t2(b, c))
    };

    let h3 = {
        let a = Arc::clone(&a);
        let c = Arc::clone(&c);
        cir_trace::spawn("t3#1142", move || t3(a, c))
    };

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
