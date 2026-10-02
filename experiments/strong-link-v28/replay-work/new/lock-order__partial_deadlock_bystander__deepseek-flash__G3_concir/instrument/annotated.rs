mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use concir_sync::Semaphore;
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    // --- shared resources -------------------------------------------------
    // `flag` is the shared variable.  The protection edge says lock `a` guards
    // it, so `flag` lives inside mutex `a`.
    let a = Arc::new(Mutex::new_named("a_mutex0#318", 0i32));
    // Lock `b` guards no variable; it exists for the two-lock handshake.
    let b = Arc::new(Mutex::new_named("b_mutex0#432", ()));

    // `sa` and `sb` start at count 0 in the design.  The linked crate releases
    // a semaphore only by handing back a permit that was acquired earlier, so
    // each semaphore is created with a single credit that main takes
    // immediately: both semaphores are back at count 0 before any worker
    // starts, and each credit is exactly the permit the matching worker hands
    // back at its `semaphore_release` step.  Leaking the boxes keeps those
    // credits alive for the whole program so they can move into the workers.
    let sa_ref: &'static Arc<Semaphore> = Box::leak(Box::new(Semaphore::new_named("sa_ref_semaphore0#1051", 1)));
    let sb_ref: &'static Arc<Semaphore> = Box::leak(Box::new(Semaphore::new_named("sb_ref_semaphore0#1133", 1)));
    let sa: &'static Semaphore = sa_ref;
    let sb: &'static Semaphore = sb_ref;

    let sa_credit = sa.acquire(); // credit worker b releases (design: b -> sa)
    let sb_credit = sb.acquire(); // credit worker a releases (design: a -> sb)

    // --- main::a ----------------------------------------------------------
    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let ha = cir_trace::spawn("forget#1536", move || {
        let mut guard_a = a1.lock().unwrap(); // s1 mutex_lock a
        sb_credit.release(); // s2 semaphore_release sb
        let sa_permit = sa.acquire(); // s3 semaphore_acquire sa
        let guard_b = b1.lock().unwrap(); // s4 mutex_lock b
        *guard_a = 1; // s5 write_shared flag = 1
        drop(guard_b); // s6 mutex_unlock b
        drop(guard_a); // s7 mutex_unlock a
        // The design has no `semaphore_release` for `sa` in this worker, so
        // its permit is kept instead of dropped (dropping would release it).
        std::mem::forget(sa_permit);
    });

    // --- main::b ----------------------------------------------------------
    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let hb = cir_trace::spawn("forget#2295", move || {
        let sb_permit = sb.acquire(); // s1 semaphore_acquire sb
        let guard_b1 = b2.lock().unwrap(); // s2 mutex_lock b
        sa_credit.release(); // s3 semaphore_release sa
        drop(guard_b1); // s4 mutex_unlock b
        let mut guard_a = a2.lock().unwrap(); // s5 mutex_lock a
        let guard_b2 = b2.lock().unwrap(); // s6 mutex_lock b
        *guard_a = 1; // s7 write_shared flag = 1
        drop(guard_b2); // s8 mutex_unlock b
        drop(guard_a); // s9 mutex_unlock a
        // Likewise, `sb` is never released by this worker.
        std::mem::forget(sb_permit);
    });

    // --- main::bystander --------------------------------------------------
    let hby = cir_trace::spawn("hby#3011", move || {
        #[allow(unused_assignments)]
        let mut tick = 0; // local `tick`, init 0
        tick = 1; // s1 assign_local tick = 1
        let _ = tick;
    });

    // --- joins ------------------------------------------------------------
    ha.join().unwrap(); // s4 join ha
    hb.join().unwrap(); // s5 join hb

    // The design never joins the bystander: it is independent and must not
    // hold up main, so the handle is dropped (the thread is detached).
    drop(hby);

    println!("DONE a=1 b=1"); // s6 return
 cir_trace::finish();}
