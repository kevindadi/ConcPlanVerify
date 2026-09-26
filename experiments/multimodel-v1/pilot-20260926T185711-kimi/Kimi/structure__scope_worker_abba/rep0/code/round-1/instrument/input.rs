use std::sync::{Arc, Mutex};
use std::thread;

fn w1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let mut done = 0;
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    done = 1;
    let _ = done;
    drop(guard_b);
    drop(guard_a);
}

fn w2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let mut done = 0;
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    done = 1;
    let _ = done;
    drop(guard_b);
    drop(guard_a);
}

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = thread::spawn(move || w1(a1, b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let h2 = thread::spawn(move || w2(a2, b2));

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
}
