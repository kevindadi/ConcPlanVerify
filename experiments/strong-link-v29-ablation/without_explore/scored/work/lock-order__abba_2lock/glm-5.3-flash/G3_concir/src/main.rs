mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn t1(a: &Arc<Mutex<()>>, b: &Arc<Mutex<()>>) -> i32 {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    let done: i32 = 1;
    drop(guard_b);
    drop(guard_a);
    done
}

fn t2(a: &Arc<Mutex<()>>, b: &Arc<Mutex<()>>) -> i32 {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    let done: i32 = 1;
    drop(guard_b);
    drop(guard_a);
    done
}

fn main() { crate::cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#491", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#529", ()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = crate::cir_trace::spawn("t1#611", move || t1(&a1, &b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let h2 = crate::cir_trace::spawn("t2#720", move || t2(&a2, &b2));

    let done1 = h1.join().unwrap();
    let done2 = h2.join().unwrap();

    println!("DONE t1={} t2={}", done1, done2);
 crate::cir_trace::finish();}
