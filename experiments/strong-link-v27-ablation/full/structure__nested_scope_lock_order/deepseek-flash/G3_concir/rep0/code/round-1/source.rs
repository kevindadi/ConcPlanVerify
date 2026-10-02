use std::sync::{Arc, Mutex};
use std::thread;

fn x1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let ga = a.lock().unwrap();
    let gb = b.lock().unwrap();
    drop(gb);
    drop(ga);
}

fn x2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let ga = a.lock().unwrap();
    let gb = b.lock().unwrap();
    drop(gb);
    drop(ga);
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

    let a_outer = a.clone();
    let b_outer = b.clone();
    let outer_h = thread::spawn(move || outer(a_outer, b_outer));

    outer_h.join().unwrap();

    println!("DONE done=1");
}
