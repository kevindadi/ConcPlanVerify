use concir_sync::Semaphore;
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // CIR `flag` (Int, init 0) lives in the mutex named by its protection
    // edge, i.e. inside lock `a`.  Lock `b` protects nothing else.
    let a = Arc::new(Mutex::new(0i32));
    let b = Arc::new(Mutex::new(()));

    // CIR semaphores `sa` and `sb` start with zero permits.  The linked crate
    // only hands out permits, so each semaphore is created with one permit
    // that `main` takes before the workers start; that pre-taken permit is
    // handed back later by the worker, which is exactly the CIR
    // `semaphore_release`.  The semaphore objects are leaked so the permits
    // can be moved into the spawned workers.
    let sa_holder: &'static Arc<Semaphore> = Box::leak(Box::new(Semaphore::new(1)));
    let sb_holder: &'static Arc<Semaphore> = Box::leak(Box::new(Semaphore::new(1)));
    let sa: &'static Semaphore = sa_holder;
    let sb: &'static Semaphore = sb_holder;

    let permit_sa = sa.acquire();
    let permit_sb = sb.acquire();

    // main::a
    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let ha = thread::spawn(move || {
        // s1: mutex_lock a
        let guard_a = a1.lock().unwrap();
        // s2: semaphore_release sa
        permit_sa.release();
        // s3: semaphore_acquire sb
        let permit = sb.acquire();
        std::mem::forget(permit);
        // s4: mutex_unlock a
        drop(guard_a);

        // s5: mutex_lock a
        let mut guard_a = a1.lock().unwrap();
        // s6: mutex_lock b
        let guard_b = b1.lock().unwrap();
        // s7: write_shared flag = 1
        *guard_a = 1;
        // s8: mutex_unlock b
        drop(guard_b);
        // s9: mutex_unlock a
        drop(guard_a);
    });

    // main::b
    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let hb = thread::spawn(move || {
        // s1: mutex_lock b
        let guard_b = b2.lock().unwrap();
        // s2: semaphore_release sb
        permit_sb.release();
        // s3: semaphore_acquire sa
        let permit = sa.acquire();
        std::mem::forget(permit);
        // s4: mutex_unlock b
        drop(guard_b);

        // s5: mutex_lock a
        let mut guard_a = a2.lock().unwrap();
        // s6: mutex_lock b
        let guard_b = b2.lock().unwrap();
        // s7: write_shared flag = 1
        *guard_a = 1;
        // s8: mutex_unlock b
        drop(guard_b);
        // s9: mutex_unlock a
        drop(guard_a);
    });

    // main::bystander: never finishes on its own; it is not joined.
    let _hby = thread::spawn(|| loop {});

    ha.join().unwrap();
    hb.join().unwrap();

    println!("DONE a=1 b=1");
}
