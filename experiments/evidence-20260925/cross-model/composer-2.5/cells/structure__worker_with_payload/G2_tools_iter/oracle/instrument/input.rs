use concir_sync::Semaphore;
use std::sync::{Arc, Mutex};
use std::thread;

fn compute() {
    let mut x = 0u64;
    for i in 0..512 {
        x = x.wrapping_mul(3).wrapping_add(i);
    }
    let _ = x;
}

fn w1(m: Arc<Semaphore>, acc: Arc<Mutex<i32>>) {
    let permit = m.acquire();
    compute();
    *acc.lock().unwrap() = 1;
    drop(permit);
}

fn w2(m: Arc<Semaphore>, acc: Arc<Mutex<i32>>) {
    let permit = m.acquire();
    compute();
    *acc.lock().unwrap() = 1;
    drop(permit);
}

fn main() {
    let m = Semaphore::new(1);
    let acc = Arc::new(Mutex::new(0));

    let m1 = Arc::clone(&m);
    let a1 = Arc::clone(&acc);
    let t1 = thread::spawn(move || w1(m1, a1));

    let m2 = Arc::clone(&m);
    let a2 = Arc::clone(&acc);
    let t2 = thread::spawn(move || w2(m2, a2));

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done=1");
}
