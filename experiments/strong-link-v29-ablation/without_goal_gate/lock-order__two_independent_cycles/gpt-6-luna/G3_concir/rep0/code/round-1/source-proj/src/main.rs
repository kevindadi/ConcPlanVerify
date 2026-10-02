use std::sync::{Arc, Mutex};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();
    drop(b_guard);
    drop(_a_guard);
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();
    drop(b_guard);
    drop(_a_guard);
}

fn t3(c: Arc<Mutex<()>>, d: Arc<Mutex<()>>) {
    let _c_guard = c.lock().unwrap();
    let d_guard = d.lock().unwrap();
    drop(d_guard);
    drop(_c_guard);
}

fn t4(c: Arc<Mutex<()>>, d: Arc<Mutex<()>>) {
    let _c_guard = c.lock().unwrap();
    let d_guard = d.lock().unwrap();
    drop(d_guard);
    drop(_c_guard);
}

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));
    let c = Arc::new(Mutex::new(()));
    let d = Arc::new(Mutex::new(()));

    let t1_handle = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        thread::spawn(move || t1(a, b))
    };
    let t2_handle = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        thread::spawn(move || t2(a, b))
    };
    let t3_handle = {
        let c = Arc::clone(&c);
        let d = Arc::clone(&d);
        thread::spawn(move || t3(c, d))
    };
    let t4_handle = {
        let c = Arc::clone(&c);
        let d = Arc::clone(&d);
        thread::spawn(move || t4(c, d))
    };

    t1_handle.join().unwrap();
    t2_handle.join().unwrap();
    t3_handle.join().unwrap();
    t4_handle.join().unwrap();

    println!("DONE done=1");
}
