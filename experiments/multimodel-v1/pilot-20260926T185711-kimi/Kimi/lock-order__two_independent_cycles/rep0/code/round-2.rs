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

    let h1 = thread::Builder::new()
        .name("t1".to_string())
        .spawn({
            let a = Arc::clone(&a);
            let b = Arc::clone(&b);
            move || t1(a, b)
        })
        .unwrap();
    let h2 = thread::Builder::new()
        .name("t2".to_string())
        .spawn({
            let a = Arc::clone(&a);
            let b = Arc::clone(&b);
            move || t2(a, b)
        })
        .unwrap();
    let h3 = thread::Builder::new()
        .name("t3".to_string())
        .spawn({
            let c = Arc::clone(&c);
            let d = Arc::clone(&d);
            move || t3(c, d)
        })
        .unwrap();
    let h4 = thread::Builder::new()
        .name("t4".to_string())
        .spawn({
            let c = Arc::clone(&c);
            let d = Arc::clone(&d);
            move || t4(c, d)
        })
        .unwrap();

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();
    h4.join().unwrap();

    println!("DONE done=1");
}
