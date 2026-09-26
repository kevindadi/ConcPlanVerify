use std::sync::{Arc, Mutex};
use std::thread;
use concir_sync::Semaphore;

fn w1(m: Arc<Mutex<i32>>, limit: Arc<Semaphore>) {
    let _permit = limit.acquire();
    let mut c = m.lock().unwrap();
    if *c < 2 {
        *c += 1;
    }
}

fn w2(m: Arc<Mutex<i32>>, limit: Arc<Semaphore>) {
    let _permit = limit.acquire();
    let mut c = m.lock().unwrap();
    if *c < 2 {
        *c += 1;
    }
}

fn main() {
    let m = Arc::new(Mutex::new(0));
    let limit = Semaphore::new(2);

    let m1 = Arc::clone(&m);
    let limit1 = Arc::clone(&limit);
    let t1 = thread::spawn(move || w1(m1, limit1));

    let m2 = Arc::clone(&m);
    let limit2 = Arc::clone(&limit);
    let t2 = thread::spawn(move || w2(m2, limit2));

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done=1");
}
