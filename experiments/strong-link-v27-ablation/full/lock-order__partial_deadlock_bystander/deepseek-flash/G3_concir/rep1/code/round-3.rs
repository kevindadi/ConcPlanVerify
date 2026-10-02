use concir_sync::{Permit, Semaphore};
use std::sync::{Arc, Mutex};
use std::thread;

// CIR `main::a`.
// `flag` (Int, init 0) is the variable protected by lock `a`; lock `b`
// protects no CIR variable.
fn a(a_lock: Arc<Mutex<i32>>, b_lock: Arc<Mutex<()>>, sa_permit: Permit, sb: Arc<Semaphore>) {
    // s1: mutex_lock a
    let guard_a = a_lock.lock().unwrap();
    // s2: semaphore_release sa
    sa_permit.release();
    // s3: semaphore_acquire sb
    let permit = sb.acquire();
    // the CIR never releases sb again, so this permit is consumed
    std::mem::forget(permit);
    // s4: mutex_unlock a
    drop(guard_a);

    // s5: mutex_lock a
    let mut guard_a = a_lock.lock().unwrap();
    // s6: mutex_lock b
    let guard_b = b_lock.lock().unwrap();
    // s7: write_shared flag = 1
    *guard_a = 1;
    // s8: mutex_unlock b
    drop(guard_b);
    // s9: mutex_unlock a
    drop(guard_a);
    // s10: return
}

// CIR `main::b`.
fn b(b_lock: Arc<Mutex<()>>, a_lock: Arc<Mutex<i32>>, sb_permit: Permit, sa: Arc<Semaphore>) {
    // s1: mutex_lock b
    let guard_b = b_lock.lock().unwrap();
    // s2: semaphore_release sb
    sb_permit.release();
    // s3: semaphore_acquire sa
    let permit = sa.acquire();
    // the CIR never releases sa again, so this permit is consumed
    std::mem::forget(permit);
    // s4: mutex_unlock b
    drop(guard_b);

    // s5: mutex_lock a
    let mut guard_a = a_lock.lock().unwrap();
    // s6: mutex_lock b
    let guard_b = b_lock.lock().unwrap();
    // s7: write_shared flag = 1
    *guard_a = 1;
    // s8: mutex_unlock b
    drop(guard_b);
    // s9: mutex_unlock a
    drop(guard_a);
    // s10: return
}

// CIR `main::bystander`: s1 goto s2, s2 goto s1.  It never finishes on its
// own, touches no shared state, and holds no lock, so it cannot prevent the
// two workers from finishing.
fn bystander() {
    loop {
        // s1 -> s2 -> s1
    }
}

fn main() {
    // Shared state: `flag` lives inside lock `a`; `b` protects nothing.
    let a_lock = Arc::new(Mutex::new(0i32));
    let b_lock = Arc::new(Mutex::new(()));

    // CIR `sa` and `sb` are counting semaphores with initial count 0.  The
    // linked crate only exposes `release` through a held permit, so each
    // semaphore is created with one permit; `main` takes that permit up front
    // (bringing the count back to 0) and hands it to the worker whose CIR body
    // performs the matching `semaphore_release`.
    let sa = Semaphore::new(1);
    let sb = Semaphore::new(1);
    let sa_permit = sa.acquire();
    let sb_permit = sb.acquire();

    // s1: spawn main::a (ha)
    let ha = {
        let a_lock = Arc::clone(&a_lock);
        let b_lock = Arc::clone(&b_lock);
        let sb = Arc::clone(&sb);
        thread::spawn(move || a(a_lock, b_lock, sa_permit, sb))
    };

    // s2: spawn main::b (hb)
    let hb = {
        let a_lock = Arc::clone(&a_lock);
        let b_lock = Arc::clone(&b_lock);
        let sa = Arc::clone(&sa);
        thread::spawn(move || b(b_lock, a_lock, sb_permit, sa))
    };

    // s3: spawn main::bystander (hby); it is never joined.
    let _hby = thread::spawn(bystander);

    // s4: join ha
    ha.join().unwrap();
    // s5: join hb
    hb.join().unwrap();
    // s6: return

    println!("DONE a=1 b=1");
}
