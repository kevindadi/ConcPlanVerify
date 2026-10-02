mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;
use concir_sync::Semaphore;

fn w1(m: &Arc<Mutex<bool>>, cv: &Arc<Condvar>, g12: &Arc<Semaphore>) {
    // s1: mutex_lock m
    let mut guard = m.lock().unwrap();
    // s2: semaphore_release g12
    let permit = g12.acquire();
    drop(permit);
    // s3/s4/s5: wait on cv while go == false, holding the lock
    while !*guard {
        guard = cv.wait(guard).unwrap();
    }
    // s6: mutex_unlock m
    drop(guard);
    // s7: return
}

fn w2(m: &Arc<Mutex<bool>>, cv: &Arc<Condvar>, g12: &Arc<Semaphore>) {
    // s1: mutex_lock m
    let mut guard = m.lock().unwrap();
    // s2: semaphore_release g12
    let permit = g12.acquire();
    drop(permit);
    // s3/s4/s5: wait on cv while go == false, holding the lock
    while !*guard {
        guard = cv.wait(guard).unwrap();
    }
    // s6: mutex_unlock m
    drop(guard);
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
    drop(p1);
    drop(p2);
    // s2: mutex_lock m
    let mut guard = m.lock().unwrap();
    // s3: write_shared go = true
    *guard = true;
    // s4: condvar_notify_all cv
    cv.notify_all();
    // s5: mutex_unlock m
    drop(guard);
    // s6: semaphore_release gN
    let permit = g_n.acquire();
    drop(permit);
    // s7: return
}

fn main() { crate::cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#1509", false));
    let cv = Arc::new(Condvar::new_named("cv_condvar0#1553"));
    let g12 = Semaphore::new_named("g12_semaphore0#1586", 0);
    let g_n = Semaphore::new_named("g_n_semaphore0#1619", 0);

    let m1 = Arc::clone(&m);
    let cv1 = Arc::clone(&cv);
    let g12_1 = Arc::clone(&g12);
    let h_w1 = crate::cir_trace::spawn("w1#1737", move || w1(&m1, &cv1, &g12_1));

    let m2 = Arc::clone(&m);
    let cv2 = Arc::clone(&cv);
    let g12_2 = Arc::clone(&g12);
    let h_w2 = crate::cir_trace::spawn("w2#1893", move || w2(&m2, &cv2, &g12_2));

    let m3 = Arc::clone(&m);
    let cv3 = Arc::clone(&cv);
    let g12_3 = Arc::clone(&g12);
    let g_n3 = Arc::clone(&g_n);
    let h_notifier = crate::cir_trace::spawn("notifier#2088", move || notifier(&m3, &cv3, &g12_3, &g_n3));

    // main: semaphore_acquire gN
    let done = g_n.acquire();
    drop(done);

    h_w1.join().unwrap();
    h_w2.join().unwrap();
    h_notifier.join().unwrap();

    println!("DONE waiters=0");
 crate::cir_trace::finish();}
