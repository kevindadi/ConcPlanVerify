use std::sync::{Arc, Mutex};
use std::thread;

fn x1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
    drop(_gb);
    drop(_ga);
}

fn x2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
    drop(_gb);
    drop(_ga);
}

fn outer(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let a1 = a.clone();
    let b1 = b.clone();
    let h1 = thread::spawn(move || x1(a1, b1));
    let a2 = a.clone();
    let b2 = b.clone();
    let h2 = thread::spawn(move || x2(a2, b2));
    h1.join().unwrap();
    h2.join().unwrap();
}

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));
    let ha = a.clone();
    let hb = b.clone();
    let h = thread::spawn(move || outer(ha, hb));
    h.join().unwrap();
    println!("DONE done=1");
}
