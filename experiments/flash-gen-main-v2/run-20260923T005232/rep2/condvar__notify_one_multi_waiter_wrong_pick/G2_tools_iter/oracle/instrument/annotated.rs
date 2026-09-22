mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", 0u32));
    let cv = Arc::new(Condvar::new_named("cv_condvar0"));
    let g12 = Arc::new((Mutex::new_named("g12_mutex0", 0u32), Condvar::new_named("g12_condvar0")));
    let gN = Arc::new((Mutex::new_named("gN_mutex0", 0u32), Condvar::new_named("gN_condvar0")));

    let m1 = Arc::clone(&m);
    let cv1 = Arc::clone(&cv);
    let g12_1 = Arc::clone(&g12);
    let gN_1 = Arc::clone(&gN);
    let w1 = cir_trace::spawn("w1", move || {
        // signal readiness
        {
            let (lock, cvar) = &*g12_1;
            let mut ready = lock.lock().unwrap();
            *ready += 1;
            if *ready == 2 {
                cvar.notify_all();
            }
        }
        // wait for notifier's go-ahead
        {
            let (lock, cvar) = &*gN_1;
            let mut go = lock.lock().unwrap();
            while *go == 0 {
                go = cvar.wait(go).unwrap();
            }
        }
        // wait on shared cv while holding lock
        let mut guard = m1.lock().unwrap();
        while *guard == 0 {
            guard = cv1.wait(guard).unwrap();
        }
    });

    let m2 = Arc::clone(&m);
    let cv2 = Arc::clone(&cv);
    let g12_2 = Arc::clone(&g12);
    let gN_2 = Arc::clone(&gN);
    let w2 = cir_trace::spawn("w2", move || {
        {
            let (lock, cvar) = &*g12_2;
            let mut ready = lock.lock().unwrap();
            *ready += 1;
            if *ready == 2 {
                cvar.notify_all();
            }
        }
        {
            let (lock, cvar) = &*gN_2;
            let mut go = lock.lock().unwrap();
            while *go == 0 {
                go = cvar.wait(go).unwrap();
            }
        }
        let mut guard = m2.lock().unwrap();
        while *guard == 0 {
            guard = cv2.wait(guard).unwrap();
        }
    });

    let m3 = Arc::clone(&m);
    let cv3 = Arc::clone(&cv);
    let g12_3 = Arc::clone(&g12);
    let gN_3 = Arc::clone(&gN);
    let notifier = cir_trace::spawn("notifier", move || {
        // wait until both waiters are ready
        {
            let (lock, cvar) = &*g12_3;
            let mut ready = lock.lock().unwrap();
            while *ready < 2 {
                ready = cvar.wait(ready).unwrap();
            }
        }
        // release waiters to proceed to cv wait
        {
            let (lock, cvar) = &*gN_3;
            let mut go = lock.lock().unwrap();
            *go = 1;
            cvar.notify_all();
        }
        // take lock, wake all waiters, release lock
        {
            let mut guard = m3.lock().unwrap();
            *guard = 1;
            cv3.notify_all();
        }
    });

    w1.join().unwrap();
    w2.join().unwrap();
    notifier.join().unwrap();

    println!("DONE waiters=0");
 cir_trace::finish();}
