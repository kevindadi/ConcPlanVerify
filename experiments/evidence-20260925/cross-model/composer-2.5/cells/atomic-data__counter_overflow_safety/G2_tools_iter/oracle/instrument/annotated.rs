mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use concir_sync::Semaphore;
use std::sync::{Arc};
use std::thread;

fn w1(m: Arc<Semaphore>, c: Arc<Mutex<u8>>) {
    let _permit = m.acquire();
    let mut guard = c.lock().unwrap();
    if *guard < 1 {
        *guard += 1;
    }
}

fn w2(m: Arc<Semaphore>, c: Arc<Mutex<u8>>) {
    let _permit = m.acquire();
    let mut guard = c.lock().unwrap();
    if *guard < 1 {
        *guard += 1;
    }
}

fn main() { cir_trace::init();
    let m = Semaphore::new_named("m_semaphore0", 1);
    let c = Arc::new(Mutex::new_named("c_mutex0", 0u8));

    let supervisor = cir_trace::spawn("supervisor", {
        let m = Arc::clone(&m);
        let c = Arc::clone(&c);
        move || {
            let h1 = cir_trace::spawn("h1", {
                let m = Arc::clone(&m);
                let c = Arc::clone(&c);
                move || w1(m, c)
            });
            let h2 = cir_trace::spawn("h2", {
                let m = Arc::clone(&m);
                let c = Arc::clone(&c);
                move || w2(m, c)
            });
            h1.join().unwrap();
            h2.join().unwrap();
        }
    });

    supervisor.join().unwrap();
    println!("DONE done=1");
 cir_trace::finish();}
