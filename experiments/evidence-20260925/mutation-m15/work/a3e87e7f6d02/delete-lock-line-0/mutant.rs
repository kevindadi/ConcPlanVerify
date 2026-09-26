use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    let a_outer = Arc::clone(&a);
    let b_outer = Arc::clone(&b);

    let h_outer = thread::spawn(move || {
        outer(a_outer, b_outer);
    });

    h_outer.join().unwrap();

    println!("DONE done=1");
}

fn outer(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);

    let h_x1 = thread::spawn(move || {
        x1(a1, b1);
    });
    let h_x2 = thread::spawn(move || {
        x2(a2, b2);
    });

    h_x1.join().unwrap();
    h_x2.join().unwrap();
}

fn x1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
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
