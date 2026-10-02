mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    drop(guard_b);
    drop(guard_a);
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    drop(guard_b);
    drop(guard_a);
}

fn t3(c: Arc<Mutex<()>>, d: Arc<Mutex<()>>) {
    let guard_c = c.lock().unwrap();
    let guard_d = d.lock().unwrap();
    drop(guard_d);
    drop(guard_c);
}

fn t4(c: Arc<Mutex<()>>, d: Arc<Mutex<()>>) {
    let guard_c = c.lock().unwrap();
    let guard_d = d.lock().unwrap();
    drop(guard_d);
    drop(guard_c);
}

fn main() { crate::cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0#731", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0#769", ()));
    let c = Arc::new(Mutex::new_named("c_mutex0#807", ()));
    let d = Arc::new(Mutex::new_named("d_mutex0#845", ()));

    let a_t1 = Arc::clone(&a);
    let b_t1 = Arc::clone(&b);
    let a_t2 = Arc::clone(&a);
    let b_t2 = Arc::clone(&b);
    let c_t3 = Arc::clone(&c);
    let d_t3 = Arc::clone(&d);
    let c_t4 = Arc::clone(&c);
    let d_t4 = Arc::clone(&d);

    let h1 = crate::cir_trace::spawn("t1#1118", move || t1(a_t1, b_t1));
    let h2 = crate::cir_trace::spawn("t2#1170", move || t2(a_t2, b_t2));
    let h3 = crate::cir_trace::spawn("t3#1222", move || t3(c_t3, d_t3));
    let h4 = crate::cir_trace::spawn("t4#1274", move || t4(c_t4, d_t4));

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();
    h4.join().unwrap();

    println!("DONE done=1");
 crate::cir_trace::finish();}
