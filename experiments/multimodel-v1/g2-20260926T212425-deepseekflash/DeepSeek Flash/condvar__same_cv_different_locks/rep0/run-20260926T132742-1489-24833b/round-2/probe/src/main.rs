use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use concir_sync::Semaphore;

fn main() {
    // m1 and m2 are the two waiters' own locks.
    let m1 = Arc::new(Mutex::new(false));
    let m2 = Arc::new(Mutex::new(false));
    // cv1 is paired with m1, cv2 is paired with m2.
    let cv1 = Arc::new(Condvar::new());
    let cv2 = Arc::new(Condvar::new());
    // ready counts how many waiters have announced themselves.
    let ready = Semaphore::new(0);

    let m1_w1 = Arc::clone(&m1);
    let cv1_w1 = Arc::clone(&cv1);
    let ready_w1 = Arc::clone(&ready);

    let m2_w2 = Arc::clone(&m2);
    let cv2_w2 = Arc::clone(&cv2);
    let ready_w2 = Arc::clone(&ready);

    let m1_n = Arc::clone(&m1);
    let m2_n = Arc::clone(&m2);
    let cv1_n = Arc::clone(&cv1);
    let cv2_n = Arc::clone(&cv2);
    let ready_n = Arc::clone(&ready);

    // w1: hold m1, announce, wait for notification while holding m1.
    let w1 = thread::spawn(move || {
        let mut guard = m1_w1.lock().unwrap();
        // Announce that we are about to wait.
        ready_w1.acquire().release();
        // Wait for the notification while holding m1.
        while !*guard {
            guard = cv1_w1.wait(guard).unwrap();
        }
    });

    // w2: hold m2, announce, wait for notification while holding m2.
    let w2 = thread::spawn(move || {
        let mut guard = m2_w2.lock().unwrap();
        ready_w2.acquire().release();
        while !*guard {
            guard = cv2_w2.wait(guard).unwrap();
        }
    });

    // notifier: wait until both waiters have announced, then wake them
    // while holding each lock.
    let notifier = thread::spawn(move || {
        // Wait for both announcements.
        ready_n.acquire().release();
        ready_n.acquire().release();

        // Wake w1 while holding m1.
        {
            let mut g1 = m1_n.lock().unwrap();
            *g1 = true;
            cv1_n.notify_all();
        }

        // Wake w2 while holding m2.
        {
            let mut g2 = m2_n.lock().unwrap();
            *g2 = true;
            cv2_n.notify_all();
        }
    });

    w1.join().unwrap();
    w2.join().unwrap();
    notifier.join().unwrap();

    println!("DONE done=1");
}
