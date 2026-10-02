use std::sync::{Arc, Mutex};

fn main() {
    let m: Arc<Mutex<i32>> = Arc::new(Mutex::new(0));

    let m1 = Arc::clone(&m);
    let h1 = std::thread::spawn(move || {
        let mut guard = m1.lock().unwrap();
        *guard = *guard + 1;
        drop(guard);
    });

    let m2 = Arc::clone(&m);
    let h2 = std::thread::spawn(move || {
        let mut guard = m2.lock().unwrap();
        *guard = *guard + 1;
        drop(guard);
    });

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
}
