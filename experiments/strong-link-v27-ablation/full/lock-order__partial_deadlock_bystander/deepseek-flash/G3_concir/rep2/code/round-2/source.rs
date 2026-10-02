use concir_sync::Semaphore;
use std::sync::{Arc, Mutex};
use std::thread;

/// `flag` is the shared variable; the design protects it with lock `a`.
struct Shared {
    flag: i32,
}

fn main() {
    // Shared resources: lock `a` (protects `flag`), lock `b`, semaphores `sa`
    // and `sb`. Both semaphores start at count 0 in the design.
    let a = Arc::new(Mutex::new(Shared { flag: 0 }));
    let b = Arc::new(Mutex::new(()));
    let sa = Semaphore::new(1);
    let sb = Semaphore::new(1);

    // The linked crate can only release a permit it already holds, so take each
    // semaphore's single credit right here: both semaphores are back to count 0
    // before any worker starts, and the credits are exactly the permits the two
    // workers hand back at their `semaphore_release` steps.
    let sa_credit = sa.acquire();
    let sb_credit = sb.acquire();

    // main::a
    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let sa1 = Arc::clone(&sa);
    let ha = thread::spawn(move || {
        let mut guard_a = a1.lock().unwrap(); // s1 mutex_lock a
        sb_credit.release(); // s2 semaphore_release sb
        let sa_permit = sa1.acquire(); // s3 semaphore_acquire sa
        let guard_b = b1.lock().unwrap(); // s4 mutex_lock b
        guard_a.flag = 1; // s5 write_shared flag = 1
        drop(guard_b); // s6 mutex_unlock b
        drop(guard_a); // s7 mutex_unlock a
        // The design never releases `sa`, so neither does this worker.
        std::mem::forget(sa_permit);
    });

    // main::b
    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let sb2 = Arc::clone(&sb);
    let hb = thread::spawn(move || {
        let sb_permit = sb2.acquire(); // s1 semaphore_acquire sb
        let guard_b1 = b2.lock().unwrap(); // s2 mutex_lock b
        sa_credit.release(); // s3 semaphore_release sa
        drop(guard_b1); // s4 mutex_unlock b
        let mut guard_a = a2.lock().unwrap(); // s5 mutex_lock a
        let guard_b2 = b2.lock().unwrap(); // s6 mutex_lock b
        guard_a.flag = 1; // s7 write_shared flag = 1
        drop(guard_b2); // s8 mutex_unlock b
        drop(guard_a); // s9 mutex_unlock a
        // The design never releases `sb`, so neither does this worker.
        std::mem::forget(sb_permit);
    });

    // main::bystander — short-lived, independent of the handshake.
    let hby = thread::spawn(move || {
        let mut tick = 0; // local `tick`, declared with init 0
        tick = 1; // s1 assign_local tick = 1
        let _ = tick;
    });

    ha.join().unwrap(); // s4 join ha
    hb.join().unwrap(); // s5 join hb

    // The bystander is deliberately not joined; it never blocks the workers.
    drop(hby);

    println!("DONE a=1 b=1");
}
