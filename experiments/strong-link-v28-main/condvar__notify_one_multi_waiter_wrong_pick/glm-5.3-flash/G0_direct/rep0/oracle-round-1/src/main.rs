mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc};
use std::thread;

// A simple counting semaphore built from a mutex + condition variable.
struct Semaphore {
    count: Mutex<i64>,
    cv: Condvar,
}

impl Semaphore {
    fn new(initial: i64) -> Self {
        Semaphore {
            count: Mutex::new(initial),
            cv: Condvar::new(),
        }
    }

    fn acquire(&self) {
        let mut count = self.count.lock().unwrap();
        while *count <= 0 {
            count = self.cv.wait(count).unwrap();
        }
        *count -= 1;
    }

    fn release(&self) {
        let mut count = self.count.lock().unwrap();
        *count += 1;
        drop(count);
        self.cv.notify_one();
    }
}

// Shared state: the lock m guarding the permit counter, and the condition variable cv.
struct State {
    m: Mutex<usize>,
    cv: Condvar,
}

fn main() { cir_trace::init();
    let state = Arc::new(State {
        m: Mutex::new_named("m#955", 0), // permit counter, starts at 0
        cv: Condvar::new_named("cv#1015"),
    });
    let g12 = Arc::new(Semaphore::new(0)); // signals when both waiters are ready to wait
    let gN = Arc::new(Semaphore::new(0)); // signals when the notifier has finished
    let finished = Arc::new(AtomicUsize::new(0));

    let mut handles = Vec::new();

    // Waiter w1
    {
        let state = Arc::clone(&state);
        let g12 = Arc::clone(&g12);
        let finished = Arc::clone(&finished);
        handles.push(thread::spawn(move || {
            // R4: hold the lock while waiting on the condition variable.
            let guard = state.m.lock().unwrap();
            // R6: tell the notifier this waiter is ready to wait.
            g12.release();
            // R2: block until told to proceed.
            let _guard = state
                .cv
                .wait_while(guard, |permit: &mut usize| *permit == 0)
                .unwrap();
            { let __cpv = finished.fetch_add(1, Ordering::SeqCst); cir_trace::record_value("finished#1223", (finished.load(std::sync::atomic::Ordering::SeqCst)) as i64); __cpv };
        }));
    }

    // Waiter w2
    {
        let state = Arc::clone(&state);
        let g12 = Arc::clone(&g12);
        let finished = Arc::clone(&finished);
        handles.push(thread::spawn(move || {
            let guard = state.m.lock().unwrap();
            g12.release();
            let _guard = state
                .cv
                .wait_while(guard, |permit: &mut usize| *permit == 0)
                .unwrap();
            { let __cpv = finished.fetch_add(1, Ordering::SeqCst); cir_trace::record_value("finished#1223", (finished.load(std::sync::atomic::Ordering::SeqCst)) as i64); __cpv };
        }));
    }

    // Notifier
    {
        let state = Arc::clone(&state);
        let g12 = Arc::clone(&g12);
        let gN = Arc::clone(&gN);
        handles.push(thread::spawn(move || {
            // R6: only proceed once both waiters are ready to wait.
            g12.acquire();
            g12.acquire();
            // R5: take the lock before waking the waiters...
            let mut permit = state.m.lock().unwrap();
            *permit += 1;
            // R3 + R7: wake every blocked waiter, not just one.
            state.cv.notify_all();
            // R5: ...and release the lock afterwards.
            drop(permit);
            // Tell main the notifier is done.
            gN.release();
        }));
    }

    // Main waits for the notifier to finish, then joins everyone (R1, R8).
    gN.acquire();
    for handle in handles {
        handle.join().unwrap();
    }

    // R9 + R10: every waiter completed, so print the required line.
    let remaining = 2 - { let __cpv = finished.load(Ordering::SeqCst); cir_trace::record_value("finished#1223", (__cpv) as i64); __cpv };
    println!("DONE waiters={}", remaining);
 cir_trace::finish();}
