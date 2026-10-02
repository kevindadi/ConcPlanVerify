use std::sync::{Arc, Mutex};
use std::thread;

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
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    let handle1 = thread::spawn({
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        move || w1(a, b)
    });
    let handle2 = thread::spawn({
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        move || w2(a, b)
    });

    handle1.join().unwrap();
    handle2.join().unwrap();

    println!("DONE done=1");
}
