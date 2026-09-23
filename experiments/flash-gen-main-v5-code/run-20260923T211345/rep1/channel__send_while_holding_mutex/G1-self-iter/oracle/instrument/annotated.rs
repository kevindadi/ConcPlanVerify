mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;
use concir_sync::Semaphore;

fn main() { cir_trace::init();
    let lock = Arc::new(Mutex::new_named("lock_mutex0", ()));

    // Rendezvous channels: each semaphore starts with one permit so the
    // first party to arrive can acquire it and release it to wake the peer.
    let ch1_send = Semaphore::new_named("ch1_send_semaphore0", 1);
    let ch1_recv = Semaphore::new_named("ch1_recv_semaphore0", 1);
    let ch2_send = Semaphore::new_named("ch2_send_semaphore0", 1);
    let ch2_recv = Semaphore::new_named("ch2_recv_semaphore0", 1);

    let lock_s = Arc::clone(&lock);
    let lock_r = Arc::clone(&lock);

    let ch1_send_s = Arc::clone(&ch1_send);
    let ch1_recv_s = Arc::clone(&ch1_recv);
    let ch2_send_s = Arc::clone(&ch2_send);
    let ch2_recv_s = Arc::clone(&ch2_recv);

    let ch1_send_r = Arc::clone(&ch1_send);
    let ch1_recv_r = Arc::clone(&ch1_recv);
    let ch2_send_r = Arc::clone(&ch2_send);
    let ch2_recv_r = Arc::clone(&ch2_recv);

    let s = cir_trace::spawn("s", move || {
        {
            let _g = lock_s.lock().unwrap();
        }

        // Rendezvous on ch1.
        let p = ch1_send_s.acquire();
        p.release();
        let q = ch1_recv_s.acquire();
        q.release();

        // Exchange value over ch2.
        let p = ch2_send_s.acquire();
        p.release();
        let q = ch2_recv_s.acquire();
        q.release();

        {
            let _g = lock_s.lock().unwrap();
        }
    });

    let r = cir_trace::spawn("r", move || {
        {
            let _g = lock_r.lock().unwrap();
        }

        // Rendezvous on ch1.
        let p = ch1_send_r.acquire();
        p.release();
        let q = ch1_recv_r.acquire();
        q.release();

        // Exchange value over ch2.
        let p = ch2_send_r.acquire();
        p.release();
        let q = ch2_recv_r.acquire();
        q.release();

        {
            let _g = lock_r.lock().unwrap();
        }
    });

    s.join().unwrap();
    r.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
