use std::sync::{Arc, Mutex};
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

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));
    let c = Arc::new(Mutex::new(()));
    let d = Arc::new(Mutex::new(()));
    let mut done = 0;

    let h1 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        thread::spawn(move || t1(a, b))
    };
    let h2 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        thread::spawn(move || t2(a, b))
    };
    let h3 = {
        let c = Arc::clone(&c);
        let d = Arc::clone(&d);
        thread::spawn(move || t3(c, d))
    };
    let h4 = {
        let c = Arc::clone(&c);
        let d = Arc::clone(&d);
        thread::spawn(move || t4(c, d))
    };

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();
    h4.join().unwrap();

    done = 1;
    println!("DONE done={}", done);
}
