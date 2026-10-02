mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) -> i32 {
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
    let work: i32 = 1;
    work
}
fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) -> i32 {
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
    let work: i32 = 1;
    work
}
fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#375", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#413", ()));
    let a1 = Arc::clone(&a); let b1 = Arc::clone(&b);
    let h1 = cir_trace::spawn("t1#490", move || t1(a1, b1));
    let a2 = Arc::clone(&a); let b2 = Arc::clone(&b);
    let h2 = cir_trace::spawn("t2#597", move || t2(a2, b2));
    let t1 = h1.join().unwrap(); let t2 = h2.join().unwrap();
    println!("DONE t1={} t2={}", t1, t2);
 cir_trace::finish();}
