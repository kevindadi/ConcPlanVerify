// SHADOW control: the flag protected by the CIR lock `m` stays false, while an
// unrelated mutex `other` holds true and the program prints DONE ready=true.
// Only the storage the CIR binds to `ready` (through `m`) may satisfy the goal.
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn waiter(m: Arc<Mutex<bool>>, _cv: Arc<Condvar>) {
    let _ready = m.lock().unwrap();
}
fn notifier(m: Arc<Mutex<bool>>, other: Arc<Mutex<bool>>, cv: Arc<Condvar>) {
    let _r = other.lock().unwrap();
    let _ready = m.lock().unwrap();
    cv.notify_one();
}
fn main() {
    let m = Arc::new(Mutex::new(false));
    let other = Arc::new(Mutex::new(true));
    let cv = Arc::new(Condvar::new());
    let m1 = Arc::clone(&m); let o1 = Arc::clone(&other); let c1 = Arc::clone(&cv);
    let w = thread::spawn(move || waiter(m1, c1));
    let m2 = Arc::clone(&m); let o2 = Arc::clone(&other); let c2 = Arc::clone(&cv);
    let n = thread::spawn(move || notifier(m2, o2, c2));
    w.join().unwrap();
    n.join().unwrap();
    println!("DONE ready=true");
}
