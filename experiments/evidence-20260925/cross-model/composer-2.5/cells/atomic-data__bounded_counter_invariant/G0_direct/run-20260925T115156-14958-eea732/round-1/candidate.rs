use concir_sync::Semaphore;
use std::sync::{Arc, Mutex};
use std::thread;

fn w1(m: Arc<Semaphore>, c: Arc<Mutex<i32>>) {
    let _permit = m.acquire();
    let mut guard = c.lock().unwrap();
    *guard += 1;
    assert!(*guard >= 0 && *guard <= 2);
}

fn w2(m: Arc<Semaphore>, c: Arc<Mutex<i32>>) {
    let _permit = m.acquire();
    let mut guard = c.lock().unwrap();
    *guard += 1;
    assert!(*guard >= 0 && *guard <= 2);
}

fn main() {
    let m = Semaphore::new(1);
    let c = Arc::new(Mutex::new(0_i32));

    let m1 = Arc::clone(&m);
    let c1 = Arc::clone(&c);
    let m2 = Arc::clone(&m);
    let c2 = Arc::clone(&c);

    let h1 = thread::spawn(move || w1(m1, c1));
    let h2 = thread::spawn(move || w2(m2, c2));

    h1.join().unwrap();
    h2.join().unwrap();

    let done = 1;
    println!("DONE done={}", done);
}
