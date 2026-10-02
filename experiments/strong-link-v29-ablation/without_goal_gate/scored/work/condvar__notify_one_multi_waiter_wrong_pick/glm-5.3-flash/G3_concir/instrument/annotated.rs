mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use concir_sync::Semaphore;
use std::sync::{Arc};
use std::thread;

fn w1(m: &Arc<Mutex<bool>>, cv: &Arc<Condvar>, g12: &Arc<Semaphore>) {
    // s1: mutex_lock m
    let mut go_guard = m.lock().unwrap();
    // s2: semaphore_release g12
    g12.acquire().release();
    // s3/s4/s5: branch on go, condvar_wait while go == false
    while !*go_guard {
        go_guard = cv.wait(go_guard).unwrap();
    }
    // s6: mutex_unlock m
    drop(go_guard);
    // s7: return
}

fn w2(m: &Arc<Mutex<bool>>, cv: &Arc<Condvar>, g12: &Arc<Semaphore>) {
    // s1: mutex_lock m
    let mut go_guard = m.lock().unwrap();
    // s2: semaphore_release g12
    g12.acquire().release();
    // s3/s4/s5: branch on go, condvar_wait while go == false
    while !*go_guard {
        go_guard = cv.wait(go_guard).unwrap();
    }
    // s6: mutex_unlock m
    drop(go_guard);
    // s7: return
}

fn notifier(
    m: &Arc<Mutex<bool>>,
    cv: &Arc<Condvar>,
    g12: &Arc<Semaphore>,
    g_n: &Arc<Semaphore>,
) {
    // s1: semaphore_acquire g12 (count 2)
    let p1 = g12.acquire();
    let p2 = g12.acquire();
    // s2: mutex_lock m
    let mut go_guard = m.lock().unwrap();
    // s3: write_shared go = true
    *go_guard = true;
    // s4: condvar_notify_all cv
    cv.notify_all();
    // s5: mutex_unlock m
    drop(go_guard);
    // s6: semaphore_release gN
    let pn = g_n.acquire();
    drop(pn);
    drop(p1);
    drop(p2);
    // s7: return
}

fn main() { crate::cir_trace::init();
    // resources: m (Mutex guarding var go), cv (Condvar), g12, gN (semaphores)
    let m: Arc<Mutex<bool>> = Arc::new(Mutex::new_named("m_mutex0#1592", false)); // var go, init false, protected by m
    let cv: Arc<Condvar> = Arc::new(Condvar::new_named("cv_condvar0#1688"));
    let g12: Arc<Semaphore> = Semaphore::new_named("g12_semaphore0#1737", 0);
    let g_n: Arc<Semaphore> = Semaphore::new_named("g_n_semaphore0#1786", 0);

    // scope: start w1, w2, notifier
    let h1 = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        let g12 = Arc::clone(&g12);
        crate::cir_trace::spawn("w1#1957", move || w1(&m, &cv, &g12))
    };
    let h2 = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        let g12 = Arc::clone(&g12);
        crate::cir_trace::spawn("w2#2130", move || w2(&m, &cv, &g12))
    };
    let h3 = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        let g12 = Arc::clone(&g12);
        let g_n = Arc::clone(&g_n);
        crate::cir_trace::spawn("notifier#2339", move || notifier(&m, &cv, &g12, &g_n))
    };

    // semaphore_acquire gN
    let pn = g_n.acquire();

    // join every spawned thread
    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    drop(pn);

    // terminal line
    println!("DONE waiters=0");
 crate::cir_trace::finish();}
