use std::sync::{Arc, Mutex};

fn w1(a: &Mutex<()>, b: &Mutex<()>) {
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
    drop(_gb);
    drop(_ga);
}

fn w2(a: &Mutex<()>, b: &Mutex<()>) {
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
    drop(_gb);
    drop(_ga);
}

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = std::thread::spawn(move || w1(&a1, &b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let h2 = std::thread::spawn(move || w2(&a2, &b2));

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
}
