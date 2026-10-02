use concir_sync::Semaphore;
use std::sync::{Arc, Mutex, Condvar};
use std::thread;

fn main() {
    // Shared condition predicate, protected by mutex m.
    let m = Arc::new(Mutex::new(false));
    let cv = Arc::new(Condvar::new());
    let g12 = Semaphore::new(0);
    let gN = Semaphore::new(0);

    // w1
    let m1 = m.clone();
    let cv1 = cv.clone();
    let g12_1 = g12.clone();
    let gN_1 = gN.clone();
    let w1 = thread::spawn(move || {
        let mut guard = m1.lock().unwrap();
        g12_1.release(); // semaphore_release g12
        while !*guard {
            guard = cv1.wait(guard).unwrap();
        }
        drop(guard); // mutex_unlock m
        gN_1.release(); // semaphore_release gN
    });

    // w2
    let m2 = m.clone();
    let cv2 = cv.clone();
    let g12_2 = g12.clone();
    let gN_2 = gN.clone();
    let w2 = thread::spawn(move || {
        let mut guard = m2.lock().unwrap();
        g12_2.release(); // semaphore_release g12
        while !*guard {
            guard = cv2.wait(guard).unwrap();
        }
        drop(guard); // mutex_unlock m
        gN_2.release(); // semaphore_release gN
    });

    // notifier
    let m3 = m.clone();
    let cv3 = cv.clone();
    let g12_3 = g12.clone();
    let gN_3 = gN.clone();
    let notifier = thread::spawn(move || {
        let _p1 = g12_3.acquire(); // semaphore_acquire g12
        let _p2 = g12_3.acquire(); // semaphore_acquire g12
        let mut guard = m3.lock().unwrap();
        *guard = true; // predicate to avoid spurious wakeups
        cv3.notify_all(); // condvar_notify_all cv
        drop(guard); // mutex_unlock m
        let _p3 = gN_3.acquire(); // semaphore_acquire gN
        let _p4 = gN_3.acquire(); // semaphore_acquire gN
    });

    w1.join().unwrap();
    w2.join().unwrap();
    notifier.join().unwrap();

    println!("DONE waiters=0");
}
