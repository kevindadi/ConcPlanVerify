use std::sync::{Arc, Mutex};
fn w(a: Arc<Mutex<i32>>, b: Arc<Mutex<i32>>) {
    let ga = a.lock().unwrap();
    drop(ga);
    let _b = b;
}
fn main() {
    let a = Arc::new(Mutex::new(0));
    let b = Arc::new(Mutex::new(0));
    let (a1, b1) = (Arc::clone(&a), Arc::clone(&b));
    let h = std::thread::spawn(move || w(a1, b1));
    h.join().unwrap();
}
