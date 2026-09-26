use concir_sync::Semaphore;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Gate {
    go: bool,
    done: bool,
}

fn w1(m1: &Mutex<Gate>, cv: &Condvar, announce: impl FnOnce()) {
    let mut guard = m1.lock().unwrap();
    announce();
    while !guard.go {
        guard = cv.wait(guard).unwrap();
    }
    guard.done = true;
    cv.notify_all();
}

fn w2(
    m1: &Mutex<Gate>,
    m2: &Mutex<bool>,
    cv: &Condvar,
    announce: impl FnOnce(),
    entering: impl FnOnce(),
) {
    announce();
    {
        let mut guard = m1.lock().unwrap();
        while !guard.done {
            guard = cv.wait(guard).unwrap();
        }
    }
    let mut guard = m2.lock().unwrap();
    entering();
    while !*guard {
        guard = cv.wait(guard).unwrap();
    }
}

fn notifier(m1: &Mutex<Gate>, m2: &Mutex<bool>, cv: &Condvar, ready: &Arc<Semaphore>) {
    let _ann_a = ready.acquire();
    let _ann_b = ready.acquire();
    {
        let mut guard = m1.lock().unwrap();
        guard.go = true;
        cv.notify_all();
    }
    let _entered = ready.acquire();
    {
        let mut guard = m2.lock().unwrap();
        *guard = true;
        cv.notify_all();
    }
}

fn main() {
    let m1 = Mutex::new(Gate {
        go: false,
        done: false,
    });
    let m2 = Mutex::new(false);
    let cv = Condvar::new();
    let ready = Semaphore::new(3);
    let ann1 = ready.acquire();
    let ann2 = ready.acquire();
    let entering = ready.acquire();

    thread::scope(|scope| {
        scope.spawn(|| {
            w1(&m1, &cv, || ann1.release());
        });
        scope.spawn(|| {
            w2(
                &m1,
                &m2,
                &cv,
                || ann2.release(),
                || entering.release(),
            );
        });
        scope.spawn(|| {
            notifier(&m1, &m2, &cv, &ready);
        });
    });

    println!("DONE done=1");
}
