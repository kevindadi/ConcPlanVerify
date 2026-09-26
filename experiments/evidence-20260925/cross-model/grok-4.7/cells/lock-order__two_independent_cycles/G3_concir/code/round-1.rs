use std::sync::{Arc, Mutex};
use std::thread;

fn println() {
    println!("DONE done=1");
}

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let ga = a.lock().unwrap();
    let gb = b.lock().unwrap();
    drop(gb);
    drop(ga);
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let ga = a.lock().unwrap();
    let gb = b.lock().unwrap();
    drop(gb);
    drop(ga);
}

fn t3(c: Arc<Mutex<()>>, d: Arc<Mutex<()>>) {
    let gc = c.lock().unwrap();
    let gd = d.lock().unwrap();
    drop(gd);
    drop(gc);
}

fn t4(c: Arc<Mutex<()>>, d: Arc<Mutex<()>>) {
    let gc = c.lock().unwrap();
    let gd = d.lock().unwrap();
    drop(gd);
    drop(gc);
}

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));
    let c = Arc::new(Mutex::new(()));
    let d = Arc::new(Mutex::new(()));

    let t1_handle = thread::spawn({
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        move || t1(a, b)
    });
    let t2_handle = thread::spawn({
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        move || t2(a, b)
    });
    let t3_handle = thread::spawn({
        let c = Arc::clone(&c);
        let d = Arc::clone(&d);
        move || t3(c, d)
    });
    let t4_handle = thread::spawn({
        let c = Arc::clone(&c);
        let d = Arc::clone(&d);
        move || t4(c, d)
    });

    t1_handle.join().unwrap();
    t2_handle.join().unwrap();
    t3_handle.join().unwrap();
    t4_handle.join().unwrap();

    println();
}
