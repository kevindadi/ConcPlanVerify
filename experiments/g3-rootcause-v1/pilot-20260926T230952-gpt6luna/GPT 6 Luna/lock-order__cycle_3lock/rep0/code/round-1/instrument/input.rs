use std::sync::{Arc, Mutex};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _a = a.lock().unwrap();
    let _b = b.lock().unwrap();
    let mut work = 0;
    work = 1;
    drop(_b);
    drop(_a);
}

fn t2(b: Arc<Mutex<()>>, c: Arc<Mutex<()>>) {
    let _b = b.lock().unwrap();
    let _c = c.lock().unwrap();
    let mut work = 0;
    work = 1;
    drop(_c);
    drop(_b);
}

fn t3(a: Arc<Mutex<()>>, c: Arc<Mutex<()>>) {
    let _a = a.lock().unwrap();
    let _c = c.lock().unwrap();
    let mut work = 0;
    work = 1;
    drop(_c);
    drop(_a);
}

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));
    let c = Arc::new(Mutex::new(()));

    let t1_handle = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        thread::spawn(move || t1(a, b))
    };
    let t2_handle = {
        let b = Arc::clone(&b);
        let c = Arc::clone(&c);
        thread::spawn(move || t2(b, c))
    };
    let t3_handle = {
        let a = Arc::clone(&a);
        let c = Arc::clone(&c);
        thread::spawn(move || t3(a, c))
    };

    t1_handle.join().unwrap();
    t2_handle.join().unwrap();
    t3_handle.join().unwrap();

    println!("DONE done=1");
}
