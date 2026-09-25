mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct WaiterState {
    announced: bool,
    notified: bool,
}

fn main() { cir_trace::init();
    let m1 = Arc::new(Mutex::new_named("m1_mutex0", WaiterState {
        announced: false,
        notified: false,
    }));
    let m2 = Arc::new(Mutex::new_named("m2_mutex0", WaiterState {
        announced: false,
        notified: false,
    }));
    let cv = Arc::new(Condvar::new_named("cv_condvar0"));
    let ready = Arc::new((Mutex::new_named("ready_mutex0", 0usize), Condvar::new_named("ready_condvar0")));

    let m1_w1 = Arc::clone(&m1);
    let cv_w1 = Arc::clone(&cv);
    let ready_w1 = Arc::clone(&ready);

    let w1 = cir_trace::spawn("w1", move || {
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

    let w2 = cir_trace::spawn("w2", move || {
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

    let notifier = cir_trace::spawn("notifier", move || {
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
 cir_trace::finish();}
