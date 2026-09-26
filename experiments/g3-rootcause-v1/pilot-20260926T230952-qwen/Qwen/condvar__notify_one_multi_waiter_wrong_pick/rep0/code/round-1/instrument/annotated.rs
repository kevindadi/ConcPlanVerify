mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use concir_sync::Semaphore;

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", false)); // 'ready' protected by 'm', init false
    let cv = Arc::new(Condvar::new_named("cv_condvar0"));
    let g12 = Arc::new(Semaphore::new_named("g12_semaphore0", 0));
    let gN = Arc::new(Semaphore::new_named("gN_semaphore0", 0));

    let m_w1 = Arc::clone(&m);
    let cv_w1 = Arc::clone(&cv);
    let g12_w1 = Arc::clone(&g12);

    let w1_handle = cir_trace::spawn("w1_handle", move || {
        // s1: semaphore_release main::g12
        let permit = g12_w1.acquire();
        permit.release();

        // s2: mutex_lock main::m
        let mut guard = m_w1.lock().unwrap();

        // s3-s5: branch on ready == true, else condvar_wait loop
        while !*guard {
            guard = cv_w1.wait(guard).unwrap();
        }

        // s6: mutex_unlock main::m (implicit via drop)
        drop(guard);
    });

    let m_w2 = Arc::clone(&m);
    let cv_w2 = Arc::clone(&cv);
    let g12_w2 = Arc::clone(&g12);

    let w2_handle = cir_trace::spawn("w2_handle", move || {
        // s1: semaphore_release main::g12
        let permit = g12_w2.acquire();
        permit.release();

        // s2: mutex_lock main::m
        let mut guard = m_w2.lock().unwrap();

        // s3-s5: branch on ready == true, else condvar_wait loop
        while !*guard {
            guard = cv_w2.wait(guard).unwrap();
        }

        // s6: mutex_unlock main::m (implicit via drop)
        drop(guard);
    });

    let m_n = Arc::clone(&m);
    let cv_n = Arc::clone(&cv);
    let g12_n = Arc::clone(&g12);
    let gN_n = Arc::clone(&gN);

    let notifier_handle = cir_trace::spawn("notifier_handle", move || {
        // s1: semaphore_acquire main::g12 count 2
        // Acquire two permits to ensure both waiters have released one each.
        let p1 = g12_n.acquire();
        let p2 = g12_n.acquire();
        
        // Release them early or let them drop? The CIR says acquire. 
        // In Rust Semaphore API, acquire returns a Permit. We need to hold the lock after acquiring.
        // The permits are just for synchronization. We can drop them now or later.
        // Dropping them releases the semaphore count back, but since we've already synchronized,
        // it doesn't matter for correctness of the next steps as long as we don't block again.
        // However, typically in these patterns, the acquire is just to wait for the release events.
        // Let's keep them alive until after we're done with the critical section if needed,
        // but actually, once we have acquired 2, we know both waiters passed their release point.
        // We can drop them immediately.
        drop(p1);
        drop(p2);

        // s2: mutex_lock main::m
        let mut guard = m_n.lock().unwrap();

        // s3: write_shared main::ready true
        *guard = true;

        // s4: condvar_notify_all main::cv
        cv_n.notify_all();

        // s5: mutex_unlock main::m
        drop(guard);

        // s6: semaphore_release main::gN
        let permit_gn = gN_n.acquire();
        permit_gn.release();
    });

    w1_handle.join().unwrap();
    w2_handle.join().unwrap();
    notifier_handle.join().unwrap();

    println!("DONE waiters=0");
 cir_trace::finish();}
