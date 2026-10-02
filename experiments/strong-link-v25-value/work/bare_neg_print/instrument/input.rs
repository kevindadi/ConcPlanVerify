// NEGATIVE control: the shared flag never becomes true, while the program prints
// DONE ready=true. var_eq ready == true must NOT be satisfied.
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn waiter(m: Arc<Mutex<bool>>, _cv: Arc<Condvar>) {
    let _ready = m.lock().unwrap();
}
fn notifier(m: Arc<Mutex<bool>>, cv: Arc<Condvar>) {
    let _ready = m.lock().unwrap();
    cv.notify_one();
}
fn main() {
    let m = Arc::new(Mutex::new(false));
    let cv = Arc::new(Condvar::new());
    let m1 = Arc::clone(&m);
    let c1 = Arc::clone(&cv);
    let w = thread::spawn(move || waiter(m1, c1));
    let m2 = Arc::clone(&m);
    let c2 = Arc::clone(&cv);
    let n = thread::spawn(move || notifier(m2, c2));
    w.join().unwrap();
    n.join().unwrap();
    println!("DONE ready=true");
}
