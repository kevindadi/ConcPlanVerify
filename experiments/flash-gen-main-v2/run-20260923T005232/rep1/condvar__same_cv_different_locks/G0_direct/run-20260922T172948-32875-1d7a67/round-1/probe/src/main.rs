use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct WaiterState {
    announced: bool,
    notified: bool,
}

fn main() {
    let m1 = Arc::new(Mutex::new(WaiterState {
        announced: false,
        notified: false,
    }));
    let m2 = Arc::new(Mutex::new(WaiterState {
        announced: false,
        notified: false,
    }));
    let cv = Arc::new(Condvar::new());
    let ready = Arc::new((Mutex::new(0usize), Condvar::new()));

    let m1_w1 = Arc::clone(&m1);
    let cv_w1 = Arc::clone(&cv);
    let ready_w1 = Arc::clone(&ready);

    let w1 = thread::spawn(move || {
        let mut guard = m1_w1.lock().unwrap();
        guard.announced = true;
        {
            let (lock, cvar) = &*ready_w1;
            let mut count = lock.lock().unwrap();
            *count += 1;
            cvar.notify_all();
        }
        while !guard.notified {
            guard = cv_w1.wait(guard).unwrap();
        }
    });

    let m2_w2 = Arc::clone(&m2);
    let cv_w2 = Arc::clone(&cv);
    let ready_w2 = Arc::clone(&ready);

    let w2 = thread::spawn(move || {
        let mut guard = m2_w2.lock().unwrap();
        guard.announced = true;
        {
            let (lock, cvar) = &*ready_w2;
            let mut count = lock.lock().unwrap();
            *count += 1;
            cvar.notify_all();
        }
        while !guard.notified {
            guard = cv_w2.wait(guard).unwrap();
        }
    });

    let m1_n = Arc::clone(&m1);
    let m2_n = Arc::clone(&m2);
    let cv_n = Arc::clone(&cv);
    let ready_n = Arc::clone(&ready);

    let notifier = thread::spawn(move || {
        {
            let (lock, cvar) = &*ready_n;
            let mut count = lock.lock().unwrap();
            while *count < 2 {
                count = cvar.wait(count).unwrap();
            }
        }

        let mut g1 = m1_n.lock().unwrap();
        let mut g2 = m2_n.lock().unwrap();

        g1.notified = true;
        g2.notified = true;

        cv_n.notify_all();

        drop(g1);
        drop(g2);
    });

    w1.join().unwrap();
    w2.join().unwrap();
    notifier.join().unwrap();

    println!("DONE done=1");
}
