use std::sync::{Arc, Mutex};
fn t1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) -> i32 {
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
    let work: i32 = 1;
    work
}
fn t2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) -> i32 {
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
    let work: i32 = 1;
    work
}
fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));
    let a1 = Arc::clone(&a); let b1 = Arc::clone(&b);
    let h1 = std::thread::spawn(move || t1(a1, b1));
    let a2 = Arc::clone(&a); let b2 = Arc::clone(&b);
    let h2 = std::thread::spawn(move || t2(a2, b2));
    let t1 = h1.join().unwrap(); let t2 = h2.join().unwrap();
    println!("DONE t1={} t2={}", t1, t2);
}
