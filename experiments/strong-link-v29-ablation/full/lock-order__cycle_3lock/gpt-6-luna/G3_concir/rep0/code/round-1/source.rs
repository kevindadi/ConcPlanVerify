use std::sync::{Arc, Mutex};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    drop(guard_b);
    drop(guard_a);
}

fn t2(b: Arc<Mutex<()>>, c: Arc<Mutex<()>>) {
    let guard_b = b.lock().unwrap();
    let guard_c = c.lock().unwrap();
    drop(guard_c);
    drop(guard_b);
}

fn t3(a: Arc<Mutex<()>>, c: Arc<Mutex<()>>) {
    let guard_a = a.lock().unwrap();
    let guard_c = c.lock().unwrap();
    drop(guard_c);
    drop(guard_a);
}

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));
    let c = Arc::new(Mutex::new(()));

    let handle1 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        thread::spawn(move || t1(a, b))
    };
    let handle2 = {
        let b = Arc::clone(&b);
        let c = Arc::clone(&c);
        thread::spawn(move || t2(b, c))
    };
    let handle3 = {
        let a = Arc::clone(&a);
        let c = Arc::clone(&c);
        thread::spawn(move || t3(a, c))
    };

    handle1.join().unwrap();
    handle2.join().unwrap();
    handle3.join().unwrap();

    println!("DONE done=1");
}
