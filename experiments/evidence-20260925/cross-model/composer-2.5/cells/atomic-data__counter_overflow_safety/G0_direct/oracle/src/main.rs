mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use concir_sync::Semaphore;
use std::sync::{Arc};
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

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", 0));
    let sem = Semaphore::new_named("sem_semaphore0", 1);

    let m1 = Arc::clone(&m);
    let m2 = Arc::clone(&m);
    let s1 = Arc::clone(&sem);
    let s2 = Arc::clone(&sem);

    let h1 = cir_trace::spawn("w1", move || w1(m1, s1));
    let h2 = cir_trace::spawn("w2", move || w2(m2, s2));

    h1.join().unwrap();
    h2.join().unwrap();

    let c = *m.lock().unwrap();
    println!("DONE done={}", c);
 cir_trace::finish();}
