use std::sync::{Arc, Mutex};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();

    let mut work = 0;
    work = work + 1;

    drop(b_guard);
    drop(a_guard);
}

fn t2(b: Arc<Mutex<()>>, c: Arc<Mutex<()>>) {
    let b_guard = b.lock().unwrap();
    let c_guard = c.lock().unwrap();

    let mut work = 0;
    work = work + 1;

    drop(c_guard);
    drop(b_guard);
}

fn t3(a: Arc<Mutex<()>>, c: Arc<Mutex<()>>) {
    let a_guard = a.lock().unwrap();
    let c_guard = c.lock().unwrap();

    let mut work = 0;
    work = work + 1;

    drop(c_guard);
    drop(a_guard);
}

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));
    let c = Arc::new(Mutex::new(()));

    let t1_a = Arc::clone(&a);
    let t1_b = Arc::clone(&b);
    let handle1 = thread::spawn(move || t1(t1_a, t1_b));

    let t2_b = Arc::clone(&b);
    let t2_c = Arc::clone(&c);
    let handle2 = thread::spawn(move || t2(t2_b, t2_c));

    let t3_a = Arc::clone(&a);
    let t3_c = Arc::clone(&c);
    let handle3 = thread::spawn(move || t3(t3_a, t3_c));

    handle1.join().unwrap();
    handle2.join().unwrap();
    handle3.join().unwrap();

    println!("DONE done=1");
}
