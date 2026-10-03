use std::sync::{Arc, Mutex};
fn w(m: Arc<Mutex<i32>>) { let _g = m.lock().unwrap(); }
fn main() {
    let m = Arc::new(Mutex::new(0));
    let m1 = Arc::clone(&m);
    let h = std::thread::spawn(move || w(m1));
    h.join().unwrap();
}
