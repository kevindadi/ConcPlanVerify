use concir_sync::Semaphore;
use std::sync::{Arc, Mutex};
use std::thread;

fn w1(m: Arc<Mutex<i32>>, sem: Arc<Semaphore>) {
    if let Some(_p) = sem.try_acquire() {
        let mut c = m.lock().unwrap();
        if *c < 1 {
            *c += 1;
        }
    }
}

fn w2(m: Arc<Mutex<i32>>, sem: Arc<Semaphore>) {
    if let Some(_p) = sem.try_acquire() {
        let mut c = m.lock().unwrap();
        if *c < 1 {
            *c += 1;
        }
    }
}

fn main() {
    let m = Arc::new(Mutex::new(0));
    let sem = Semaphore::new(1);

    let m1 = Arc::clone(&m);
    let m2 = Arc::clone(&m);
    let s1 = Arc::clone(&sem);
    let s2 = Arc::clone(&sem);

    let h1 = thread::spawn(move || w1(m1, s1));
    let h2 = thread::spawn(move || w2(m2, s2));

    h1.join().unwrap();
    h2.join().unwrap();

    let c = *m.lock().unwrap();
    println!("DONE done={}", c);
}
