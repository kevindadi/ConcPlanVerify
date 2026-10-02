mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use concir_sync::Semaphore;
use std::sync::{Arc};
use std::thread;

struct State {
    ready: usize,
    proceed: bool,
}

fn w1(m: Arc<Mutex<State>>, cv: Arc<Condvar>) {
    let mut state = m.lock().unwrap();
    state.ready += 1;
    cv.notify_all();

    while !state.proceed {
        state = cv.wait(state).unwrap();
    }
}

fn w2(m: Arc<Mutex<State>>, cv: Arc<Condvar>) {
    let mut state = m.lock().unwrap();
    state.ready += 1;
    cv.notify_all();

    while !state.proceed {
        state = cv.wait(state).unwrap();
    }
}

fn notifier(m: Arc<Mutex<State>>, cv: Arc<Condvar>) {
    let mut state = m.lock().unwrap();

    while state.ready < 2 {
        state = cv.wait(state).unwrap();
    }

    state.proceed = true;
    cv.notify_all();
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#815", State {
        ready: 0,
        proceed: false,
    }));
    let cv = Arc::new(Condvar::new_named("cv_condvar0#909"));
    let g12 = Semaphore::new_named("g12_semaphore0#942", 0);
    let gN = Semaphore::new_named("gN_semaphore0#974", 0);

    let w1_handle = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        cir_trace::spawn("w1#1079", move || w1(m, cv))
    };

    let w2_handle = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        cir_trace::spawn("w2#1216", move || w2(m, cv))
    };

    let notifier_handle = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        cir_trace::spawn("notifier#1359", move || notifier(m, cv))
    };

    w1_handle.join().unwrap();
    w2_handle.join().unwrap();
    notifier_handle.join().unwrap();

    let _ = (g12, gN);
    println!("DONE waiters=0");
 cir_trace::finish();}
