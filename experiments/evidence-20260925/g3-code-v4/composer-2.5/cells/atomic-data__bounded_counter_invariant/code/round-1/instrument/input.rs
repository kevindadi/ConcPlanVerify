use std::sync::{Arc, Mutex};
use std::thread;

fn w1(m: Arc<Mutex<i32>>) {
    let mut guard = m.lock().unwrap();
    *guard = *guard + 1;
}

fn w2(m: Arc<Mutex<i32>>) {
    let mut guard = m.lock().unwrap();
    *guard = *guard + 1;
}

fn main() {
    let m = Arc::new(Mutex::new(0));
    let m_w1 = Arc::clone(&m);
    let m_w2 = Arc::clone(&m);

    let h1 = thread::spawn(move || w1(m_w1));
    let h2 = thread::spawn(move || w2(m_w2));

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
}
