mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;
use concir_sync::Semaphore;

fn main() { cir_trace::init();
    // ch1: rendezvous channel (capacity 0) implemented with two semaphores.
    // Each semaphore starts with one permit so a role can acquire-then-release
    // to signal readiness, and the other role can acquire to wait.
    let ch1_send = Semaphore::new_named("ch1_send_semaphore0", 1);
    let ch1_recv = Semaphore::new_named("ch1_recv_semaphore0", 1);

    // ch2: a second channel used to signal completion / handshake.
    let ch2_send = Semaphore::new_named("ch2_send_semaphore0", 1);
    let ch2_recv = Semaphore::new_named("ch2_recv_semaphore0", 1);

    // Shared lock occasionally used by both roles.
    let lock = Arc::new(Mutex::new_named("lock_mutex0", 0u32));

    let lock_s = Arc::clone(&lock);
    let lock_r = Arc::clone(&lock);

    let s_send = Arc::clone(&ch1_send);
    let s_recv = Arc::clone(&ch1_recv);
    let s_ch2_send = Arc::clone(&ch2_send);
    let s_ch2_recv = Arc::clone(&ch2_recv);

    let r_send = Arc::clone(&ch1_send);
    let r_recv = Arc::clone(&ch1_recv);
    let r_ch2_send = Arc::clone(&ch2_send);
    let r_ch2_recv = Arc::clone(&ch2_recv);

    // Sender role: s
    let s = cir_trace::spawn("s", move || {
        // Occasionally use the shared lock, but never while waiting on ch1.
        {
            let mut g = lock_s.lock().unwrap();
            *g += 1;
        }

        // Rendezvous on ch1: signal ready, then wait for receiver.
        s_send.acquire().release();
        s_recv.acquire();

        // Use the shared lock again after the channel meeting.
        {
            let mut g = lock_s.lock().unwrap();
            *g += 1;
        }

        // Rendezvous on ch2 to confirm completion.
        s_ch2_send.acquire().release();
        s_ch2_recv.acquire();
    });

    // Receiver role: r
    let r = cir_trace::spawn("r", move || {
        // Occasionally use the shared lock, but never while waiting on ch1.
        {
            let mut g = lock_r.lock().unwrap();
            *g += 1;
        }

        // Rendezvous on ch1: wait for sender, then signal ready.
        r_send.acquire();
        r_recv.acquire().release();

        // Use the shared lock again after the channel meeting.
        {
            let mut g = lock_r.lock().unwrap();
            *g += 1;
        }

        // Rendezvous on ch2 to confirm completion.
        r_ch2_send.acquire();
        r_ch2_recv.acquire().release();
    });

    s.join().unwrap();
    r.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
