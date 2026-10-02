mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use concir_sync::Semaphore;
use std::mem;

use std::thread;

fn main() { cir_trace::init();
    // main::m, main::cv, main::g12, main::gN
    let m = Mutex::new_named("m_mutex0#170", ());
    let cv = Condvar::new_named("cv_condvar0#201");
    let g12 = Semaphore::new_named("g12_semaphore0#233", 2);
    let gN = Semaphore::new_named("gN_semaphore0#265", 2);

    // The design's roles *release* permits into g12 (w1, w2) and into gN
    // (notifier) before anything acquires from those semaphores, so each
    // releasing role must hold a permit to hand back.  Take those permits
    // here (leaving both semaphores empty, exactly as in the design) and
    // give them to the role that performs the release below.
    let w1_g12 = g12.acquire();
    let w2_g12 = g12.acquire();
    let notifier_gN1 = gN.acquire();
    let notifier_gN2 = gN.acquire();

    let m = &m;
    let cv = &cv;
    let g12 = &g12;
    let gN = &gN;

    // main: scope { w1, w2, notifier }
    thread::scope(|scope| {
        // main::w1
        scope.spawn(move || {
            let guard = m.lock().unwrap(); // mutex_lock m
            w1_g12.release(); // semaphore_release g12
            let guard = cv.wait(guard).unwrap(); // condvar_wait cv, m
            drop(guard); // mutex_unlock m
            mem::forget(gN.acquire()); // semaphore_acquire gN (permit held)
        });

        // main::w2
        scope.spawn(move || {
            let guard = m.lock().unwrap(); // mutex_lock m
            w2_g12.release(); // semaphore_release g12
            let guard = cv.wait(guard).unwrap(); // condvar_wait cv, m
            drop(guard); // mutex_unlock m
            mem::forget(gN.acquire()); // semaphore_acquire gN (permit held)
        });

        // main::notifier
        scope.spawn(move || {
            mem::forget(g12.acquire()); // semaphore_acquire g12
            mem::forget(g12.acquire()); // semaphore_acquire g12
            let guard = m.lock().unwrap(); // mutex_lock m
            cv.notify_all(); // condvar_notify_all cv
            drop(guard); // mutex_unlock m
            notifier_gN1.release(); // semaphore_release gN
            notifier_gN2.release(); // semaphore_release gN
        });
    });

    println!("DONE waiters=0");
 cir_trace::finish();}
