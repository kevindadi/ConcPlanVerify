use concir_sync::Semaphore;
use std::sync::{Arc, Mutex};
use std::thread;

fn compute() {
    let mut x: u64 = 1;
    for i in 0..512 {
        x = x.wrapping_mul(3).wrapping_add(i);
    }
    let _ = x;
}

fn w1(m: Arc<Mutex<()>>, acc: Arc<Mutex<i32>>) {
    let _guard = m.lock().unwrap();
    compute();
    *acc.lock().unwrap() = 1;
}

fn w2(m: Arc<Mutex<()>>, acc: Arc<Mutex<i32>>) {
    let _guard = m.lock().unwrap();
    compute();
    *acc.lock().unwrap() = 1;
}

fn main() {
    let _sem = Semaphore::new(2);
    let m = Arc::new(Mutex::new(()));
    let acc = Arc::new(Mutex::new(0));

    let m1 = Arc::clone(&m);
    let acc1 = Arc::clone(&acc);
    let t1 = thread::spawn(move || w1(m1, acc1));

    let m2 = Arc::clone(&m);
    let acc2 = Arc::clone(&acc);
    let t2 = thread::spawn(move || w2(m2, acc2));

    t1.join().unwrap();
    t2.join().unwrap();

    let done = *acc.lock().unwrap();
    println!("DONE done={done}");
}
