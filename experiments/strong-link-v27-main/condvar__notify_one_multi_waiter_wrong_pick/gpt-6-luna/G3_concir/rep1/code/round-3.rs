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

    thread::scope(|scope| {
        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        scope.spawn(move || w1(m, cv));

        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        scope.spawn(move || w2(m, cv));

        let m = Arc::clone(&m);
        let cv = Arc::clone(&cv);
        scope.spawn(move || notifier(m, cv));
    });

    println!("DONE waiters=0");
}
