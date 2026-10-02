use std::sync::{Arc, Mutex};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _a_guard = a.lock().unwrap();
    let _b_guard = b.lock().unwrap();
    std::hint::black_box(());
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _a_guard = a.lock().unwrap();
    let _b_guard = b.lock().unwrap();
    std::hint::black_box(());
}

fn t3(c: Arc<Mutex<()>>, d: Arc<Mutex<()>>) {
    let _c_guard = c.lock().unwrap();
    let _d_guard = d.lock().unwrap();
    std::hint::black_box(());
}

fn t4(c: Arc<Mutex<()>>, d: Arc<Mutex<()>>) {
    let _c_guard = c.lock().unwrap();
    let _d_guard = d.lock().unwrap();
    std::hint::black_box(());
}

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));
    let c = Arc::new(Mutex::new(()));
    let d = Arc::new(Mutex::new(()));

    let h1 = thread::spawn({
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        move || t1(a, b)
    });
    let h2 = thread::spawn({
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        move || t2(a, b)
    });
    let h3 = thread::spawn({
        let c = Arc::clone(&c);
        let d = Arc::clone(&d);
        move || t3(c, d)
    });
    let h4 = thread::spawn({
        let c = Arc::clone(&c);
        let d = Arc::clone(&d);
        move || t4(c, d)
    });

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();
    h4.join().unwrap();

    println!("DONE done=1");
}
