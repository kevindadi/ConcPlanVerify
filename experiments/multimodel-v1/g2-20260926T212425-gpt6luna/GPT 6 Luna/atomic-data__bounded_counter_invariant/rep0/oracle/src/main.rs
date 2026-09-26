mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use concir_sync::Semaphore;
use std::sync::{Arc};
use std::thread;

const COUNTER_MIN: u8 = 0;
const COUNTER_MAX: u8 = 2;

fn w1(m: Arc<Semaphore>, c: Arc<Mutex<u8>>) {
    let _permit = m.acquire();
    let mut counter = c.lock().unwrap();
    assert!(*counter >= COUNTER_MIN && *counter < COUNTER_MAX);
    *counter += 1;
    assert!(*counter <= COUNTER_MAX);
}

fn w2(m: Arc<Semaphore>, c: Arc<Mutex<u8>>) {
    let _permit = m.acquire();
    let mut counter = c.lock().unwrap();
    assert!(*counter >= COUNTER_MIN && *counter < COUNTER_MAX);
    *counter += 1;
    assert!(*counter <= COUNTER_MAX);
}

fn main() { cir_trace::init();
    let m = Semaphore::new_named("m_semaphore0", 1);
    let c = Arc::new(Mutex::new_named("c_mutex0", COUNTER_MIN));

    let m1 = Arc::clone(&m);
    let c1 = Arc::clone(&c);
    let worker1 = cir_trace::spawn("w1", move || w1(m1, c1));

    let m2 = Arc::clone(&m);
    let c2 = Arc::clone(&c);
    let worker2 = cir_trace::spawn("w2", move || w2(m2, c2));

    worker1.join().unwrap();
    worker2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
