mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
// NEGATIVE control: the shared flag never becomes true, while the program prints
// DONE ready=true. var_eq ready == true must NOT be satisfied.
use std::sync::{Arc};
use std::thread;

fn waiter(m: Arc<Mutex<bool>>, _cv: Arc<Condvar>) {
    let _ready = m.lock().unwrap();
}
fn notifier(m: Arc<Mutex<bool>>, cv: Arc<Condvar>) {
    let _ready = m.lock().unwrap();
    cv.notify_one();
}
fn main() { crate::cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#444", false));
    let cv = Arc::new(Condvar::new_named("cv_condvar0#488"));
    let m1 = Arc::clone(&m);
    let c1 = Arc::clone(&cv);
    let w = crate::cir_trace::spawn("waiter#567", move || waiter(m1, c1));
    let m2 = Arc::clone(&m);
    let c2 = Arc::clone(&cv);
    let n = crate::cir_trace::spawn("notifier#677", move || notifier(m2, c2));
    w.join().unwrap();
    n.join().unwrap();
    println!("DONE ready=true");
 crate::cir_trace::finish();}
