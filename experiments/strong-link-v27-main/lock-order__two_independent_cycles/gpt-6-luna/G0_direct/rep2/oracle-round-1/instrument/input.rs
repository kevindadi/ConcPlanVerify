use std::sync::{Arc, Mutex};
use std::thread;

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _a = a.lock().unwrap();
    let _b = b.lock().unwrap();
    let _work = 1 + 1;
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _a = a.lock().unwrap();
    let _b = b.lock().unwrap();
    let _work = 1 + 1;
}

fn t3(c: Arc<Mutex<()>>, d: Arc<Mutex<()>>) {
    let _c = c.lock().unwrap();
    let _d = d.lock().unwrap();
    let _work = 1 + 1;
}

fn t4(c: Arc<Mutex<()>>, d: Arc<Mutex<()>>) {
    let _c = c.lock().unwrap();
    let _d = d.lock().unwrap();
    let _work = 1 + 1;
}

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));
    let c = Arc::new(Mutex::new(()));
    let d = Arc::new(Mutex::new(()));

    let handles = vec![
        {
            let (a, b) = (Arc::clone(&a), Arc::clone(&b));
            thread::spawn(move || t1(a, b))
        },
        {
            let (a, b) = (Arc::clone(&a), Arc::clone(&b));
            thread::spawn(move || t2(a, b))
        },
        {
            let (c, d) = (Arc::clone(&c), Arc::clone(&d));
            thread::spawn(move || t3(c, d))
        },
        {
            let (c, d) = (Arc::clone(&c), Arc::clone(&d));
            thread::spawn(move || t4(c, d))
        },
    ];

    for handle in handles {
        handle.join().unwrap();
    }

    let done = 1;
    println!("DONE done={done}");
}
