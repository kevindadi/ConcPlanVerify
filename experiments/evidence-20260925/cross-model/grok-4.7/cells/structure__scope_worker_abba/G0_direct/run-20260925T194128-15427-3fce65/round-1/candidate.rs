use std::sync::{Arc, Mutex};
use std::thread;

use concir_sync::Semaphore;

fn w1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    drop(guard_b);
    drop(guard_a);
}

fn w2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    drop(guard_b);
    drop(guard_a);
}

fn main() {
    let _sem = Semaphore::new(2);
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let t1 = thread::spawn(move || w1(a1, b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let t2 = thread::spawn(move || w2(a2, b2));

    t1.join().unwrap();
    t2.join().unwrap();
    println!("DONE done=1");
}
