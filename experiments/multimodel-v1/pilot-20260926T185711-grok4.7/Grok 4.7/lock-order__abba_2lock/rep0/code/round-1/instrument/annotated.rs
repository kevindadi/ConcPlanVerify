mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0", ()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = cir_trace::spawn("t1", move || t1(a1, b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let h2 = cir_trace::spawn("t2", move || t2(a2, b2));

    let t1_done = h1.join().unwrap();
    let t2_done = h2.join().unwrap();
    println!("DONE t1={} t2={}", t1_done, t2_done);
 cir_trace::finish();}

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) -> i32 {
    let mut done = 0;
    let ga = a.lock().unwrap();
    let gb = b.lock().unwrap();
    done = 1;
    drop(gb);
    drop(ga);
    done
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) -> i32 {
    let mut done = 0;
    let ga = a.lock().unwrap();
    let gb = b.lock().unwrap();
    done = 1;
    drop(gb);
    drop(ga);
    done
}
