use std::sync::{Arc, Mutex};
use std::thread;

fn w1(a: &Arc<Mutex<()>>, b: &Arc<Mutex<()>>) {
    let a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();
    drop(b_guard);
    drop(a_guard);
}

fn w2(a: &Arc<Mutex<()>>, b: &Arc<Mutex<()>>) {
    let a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();
    drop(b_guard);
    drop(a_guard);
}

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));
    let mut done = 0;

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let t1 = thread::spawn(move || w1(&a1, &b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let t2 = thread::spawn(move || w2(&a2, &b2));

    t1.join().unwrap();
    t2.join().unwrap();

    done = 1;
    println!("DONE done={}", done);
}
