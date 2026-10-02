use concir_sync::Semaphore;
use std::sync::{Arc, Condvar, Mutex};
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

fn main() {
    let m = Arc::new(Mutex::new(State {
        ready: 0,
        proceed: false,
    }));
    let cv = Arc::new(Condvar::new());
    let g12 = Semaphore::new(0);
    let gN = Semaphore::new(0);

    let w1_handle = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        thread::spawn(move || w1(m, cv))
    };

    let w2_handle = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        thread::spawn(move || w2(m, cv))
    };

    let notifier_handle = {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        thread::spawn(move || notifier(m, cv))
    };

    w1_handle.join().unwrap();
    w2_handle.join().unwrap();
    notifier_handle.join().unwrap();

    let _ = (g12, gN);
    println!("DONE waiters=0");
}
