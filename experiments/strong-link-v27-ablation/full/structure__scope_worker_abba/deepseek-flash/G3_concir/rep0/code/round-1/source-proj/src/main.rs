use std::sync::{Arc, Mutex};
use std::thread;

fn w1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let ga = a.lock().unwrap();
    let gb = b.lock().unwrap();
    drop(gb);
    drop(ga);
}

fn w2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let ga = a.lock().unwrap();
    let gb = b.lock().unwrap();
    drop(gb);
    drop(ga);
}

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);

    let h1 = thread::spawn(move || w1(a1, b1));
    let h2 = thread::spawn(move || w2(a2, b2));

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
}
