use concir_sync::Semaphore;
use std::sync::{Arc, Condvar, Mutex};

fn w1(m: &Arc<Mutex<bool>>, cv: &Arc<Condvar>, g12: &Arc<Semaphore>, g_n: &Arc<Semaphore>) {
    // s1: mutex_lock m
    let mut guard = m.lock().unwrap();
    // s2: semaphore_release g12
    let permit = g12.acquire();
    permit.release();
    // s3: branch go == true / s4: condvar_wait / s5: goto s3
    while !*guard {
        guard = cv.wait(guard).unwrap();
    }
    // s6: mutex_unlock m
    drop(guard);
    // s7: semaphore_acquire gN
    let _p = g_n.acquire();
    // s8: return
}

fn w2(m: &Arc<Mutex<bool>>, cv: &Arc<Condvar>, g12: &Arc<Semaphore>, g_n: &Arc<Semaphore>) {
    // s1: mutex_lock m
    let mut guard = m.lock().unwrap();
    // s2: semaphore_release g12
    let permit = g12.acquire();
    permit.release();
    // s3: branch go == true / s4: condvar_wait / s5: goto s3
    while !*guard {
        guard = cv.wait(guard).unwrap();
    }
    // s6: mutex_unlock m
    drop(guard);
    // s7: semaphore_acquire gN
    let _p = g_n.acquire();
    // s8: return
}

fn notifier(m: &Arc<Mutex<bool>>, cv: &Arc<Condvar>, g12: &Arc<Semaphore>, g_n: &Arc<Semaphore>) {
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
    // s6, s7: semaphore_release gN (twice)
    let pa = g_n.acquire();
    pa.release();
    let pb = g_n.acquire();
    pb.release();
    // s8: return
}

fn main() {
    let m = Arc::new(Mutex::new(false)); // go, protected by m
    let cv = Arc::new(Condvar::new());
    let g12 = Semaphore::new(0);
    let g_n = Semaphore::new(0);

    let m1 = Arc::clone(&m);
    let cv1 = Arc::clone(&cv);
    let g12_1 = Arc::clone(&g12);
    let gn1 = Arc::clone(&g_n);
    let h1 = std::thread::spawn(move || w1(&m1, &cv1, &g12_1, &gn1));

    let m2 = Arc::clone(&m);
    let cv2 = Arc::clone(&cv);
    let g12_2 = Arc::clone(&g12);
    let gn2 = Arc::clone(&g_n);
    let h2 = std::thread::spawn(move || w2(&m2, &cv2, &g12_2, &gn2));

    let m3 = Arc::clone(&m);
    let cv3 = Arc::clone(&cv);
    let g12_3 = Arc::clone(&g12);
    let gn3 = Arc::clone(&g_n);
    let h3 = std::thread::spawn(move || notifier(&m3, &cv3, &g12_3, &gn3));

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    println!("DONE waiters=0");
}
