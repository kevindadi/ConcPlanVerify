mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use concir_sync::Semaphore;
use std::sync::{Arc};
use std::thread;

fn w1(m: Arc<Mutex<bool>>, cv: Arc<Condvar>) {
    let mut proceed = m.lock().unwrap();
    while !*proceed {
        proceed = cv.wait(proceed).unwrap();
    }
}

fn w2(m: Arc<Mutex<bool>>, cv: Arc<Condvar>) {
    let mut proceed = m.lock().unwrap();
    while !*proceed {
        proceed = cv.wait(proceed).unwrap();
    }
}

fn notifier(m: Arc<Mutex<bool>>, cv: Arc<Condvar>) {
    let mut proceed = m.lock().unwrap();
    *proceed = true;
    cv.notify_all();
}

fn main() { crate::cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#591", false));
    let cv = Arc::new(Condvar::new_named("cv_condvar0#635"));
    let g12 = Semaphore::new_named("g12_semaphore0#668", 0);
    let gN = Semaphore::new_named("gN_semaphore0#700", 0);

    let w1_m = Arc::clone(&m);
    let w1_cv = Arc::clone(&cv);
    let t1 = crate::cir_trace::spawn("w1#786", move || w1(w1_m, w1_cv));

    let w2_m = Arc::clone(&m);
    let w2_cv = Arc::clone(&cv);
    let t2 = crate::cir_trace::spawn("w2#904", move || w2(w2_m, w2_cv));

    let notifier_m = Arc::clone(&m);
    let notifier_cv = Arc::clone(&cv);
    let tn = crate::cir_trace::spawn("notifier#1034", move || notifier(notifier_m, notifier_cv));

    t1.join().unwrap();
    t2.join().unwrap();
    tn.join().unwrap();

    let _ = (g12, gN);
    println!("DONE waiters=0");
 crate::cir_trace::finish();}
