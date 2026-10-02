mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use concir_sync::{Permit, Semaphore};
use std::sync::{Arc};
use std::thread;

fn w1(m: Arc<Mutex<()>>, cv: Arc<Condvar>, g12_permit: Permit<'static>, g_n: Arc<Semaphore>) {
    let guard = m.lock().unwrap();
    g12_permit.release();
    let guard = cv.wait(guard).unwrap();
    drop(guard);

    let permit = g_n.acquire();
    drop(permit);
}

fn w2(m: Arc<Mutex<()>>, cv: Arc<Condvar>, g12_permit: Permit<'static>, g_n: Arc<Semaphore>) {
    let guard = m.lock().unwrap();
    g12_permit.release();
    let guard = cv.wait(guard).unwrap();
    drop(guard);

    let permit = g_n.acquire();
    drop(permit);
}

fn notifier(
    m: Arc<Mutex<()>>,
    cv: Arc<Condvar>,
    g12: Arc<Semaphore>,
    g_n_permit_1: Permit<'static>,
    g_n_permit_2: Permit<'static>,
) {
    let ready_1 = g12.acquire();
    let ready_2 = g12.acquire();

    let guard = m.lock().unwrap();
    cv.notify_all();
    drop(guard);

    g_n_permit_1.release();
    g_n_permit_2.release();

    drop(ready_1);
    drop(ready_2);
}

fn main() { crate::cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#1066", ()));
    let cv = Arc::new(Condvar::new_named("cv_condvar0#1107"));

    let g12 = Semaphore::new_named("g12_semaphore0#1141", 2);
    let g_n = Semaphore::new_named("g_n_semaphore0#1174", 2);

    let g12_static: &'static Arc<Semaphore> =
        Box::leak(Box::new(Arc::clone(&g12)));
    let g_n_static: &'static Arc<Semaphore> =
        Box::leak(Box::new(Arc::clone(&g_n)));

    let w1_g12_permit = g12_static.acquire();
    let w2_g12_permit = g12_static.acquire();
    let notifier_g_n_permit_1 = g_n_static.acquire();
    let notifier_g_n_permit_2 = g_n_static.acquire();

    let w1_handle = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        let g_n = Arc::clone(&g_n);
        crate::cir_trace::spawn("w1#1703", move || w1(m, cv, w1_g12_permit, g_n))
    };

    let w2_handle = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        let g_n = Arc::clone(&g_n);
        crate::cir_trace::spawn("w2#1896", move || w2(m, cv, w2_g12_permit, g_n))
    };

    let notifier_handle = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        let g12 = Arc::clone(&g12);
        crate::cir_trace::spawn("notifier#2095", move || {
            notifier(
                m,
                cv,
                g12,
                notifier_g_n_permit_1,
                notifier_g_n_permit_2,
            )
        })
    };

    w1_handle.join().unwrap();
    w2_handle.join().unwrap();
    notifier_handle.join().unwrap();

    println!("DONE waiters=0");
 crate::cir_trace::finish();}
