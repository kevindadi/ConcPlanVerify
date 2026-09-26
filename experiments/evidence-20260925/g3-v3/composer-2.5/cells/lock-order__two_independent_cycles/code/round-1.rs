use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
    drop(_gb);
    drop(_ga);
}

fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
    drop(_gb);
    drop(_ga);
}

fn t3(c: Arc<Mutex<()>>, d: Arc<Mutex<()>>) {
    let _gc = c.lock().unwrap();
    let _gd = d.lock().unwrap();
    drop(_gd);
    drop(_gc);
}

fn t4(c: Arc<Mutex<()>>, d: Arc<Mutex<()>>) {
    let _gc = c.lock().unwrap();
    let _gd = d.lock().unwrap();
    drop(_gd);
    drop(_gc);
}

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));
    let c = Arc::new(Mutex::new(()));
    let d = Arc::new(Mutex::new(()));

    let workers: [JoinHandle<()>; 4] = [
        thread::spawn({
            let a = Arc::clone(&a);
            let b = Arc::clone(&b);
            move || t1(a, b)
        }),
        thread::spawn({
            let a = Arc::clone(&a);
            let b = Arc::clone(&b);
            move || t2(a, b)
        }),
        thread::spawn({
            let c = Arc::clone(&c);
            let d = Arc::clone(&d);
            move || t3(c, d)
        }),
        thread::spawn({
            let c = Arc::clone(&c);
            let d = Arc::clone(&d);
            move || t4(c, d)
        }),
    ];

    for h in workers {
        h.join().unwrap();
    }

    println!("DONE done=1");
}
