use std::sync::{Arc, Mutex};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _a_guard = a.lock().unwrap();
    let _b_guard = b.lock().unwrap();
    let _work = 1usize.wrapping_add(1);
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _a_guard = a.lock().unwrap();
    let _b_guard = b.lock().unwrap();
    let _work = 1usize.wrapping_add(1);
}

fn t3(c: Arc<Mutex<()>>, d: Arc<Mutex<()>>) {
    let _c_guard = c.lock().unwrap();
    let _d_guard = d.lock().unwrap();
    let _work = 1usize.wrapping_add(1);
}

fn t4(c: Arc<Mutex<()>>, d: Arc<Mutex<()>>) {
    let _c_guard = c.lock().unwrap();
    let _d_guard = d.lock().unwrap();
    let _work = 1usize.wrapping_add(1);
}

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));
    let c = Arc::new(Mutex::new(()));
    let d = Arc::new(Mutex::new(()));

    let worker1 = thread::Builder::new()
        .name("t1".into())
        .spawn({
            let a = Arc::clone(&a);
            let b = Arc::clone(&b);
            move || t1(a, b)
        })
        .unwrap();

    let worker2 = thread::Builder::new()
        .name("t2".into())
        .spawn({
            let a = Arc::clone(&a);
            let b = Arc::clone(&b);
            move || t2(a, b)
        })
        .unwrap();

    let worker3 = thread::Builder::new()
        .name("t3".into())
        .spawn({
            let c = Arc::clone(&c);
            let d = Arc::clone(&d);
            move || t3(c, d)
        })
        .unwrap();

    let worker4 = thread::Builder::new()
        .name("t4".into())
        .spawn({
            let c = Arc::clone(&c);
            let d = Arc::clone(&d);
            move || t4(c, d)
        })
        .unwrap();

    worker1.join().unwrap();
    worker2.join().unwrap();
    worker3.join().unwrap();
    worker4.join().unwrap();

    println!("DONE done=1");
}
