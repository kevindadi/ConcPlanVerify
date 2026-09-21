use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    let m1 = Arc::new(Mutex::new(false));
    let m2 = Arc::new(Mutex::new(false));
    let cv = Arc::new(Condvar::new());
    let announced = Arc::new((Mutex::new(0usize), Condvar::new()));

    let m1_w = Arc::clone(&m1);
    let m2_w = Arc::clone(&m2);
    let cv_w = Arc::clone(&cv);
    let ann_w = Arc::clone(&announced);

    let w1 = thread::spawn(move || {
        let mut g = m1_w.lock().unwrap();
        {
            let (lock, cvar) = &*ann_w;
            let mut n = lock.lock().unwrap();
            *n += 1;
            cvar.notify_all();
        }
        while !*g {
            g = cv_w.wait(g).unwrap();
        }
    });

    let m1_w2 = Arc::clone(&m1);
    let m2_w2 = Arc::clone(&m2);
    let cv_w2 = Arc::clone(&cv);
    let ann_w2 = Arc::clone(&announced);

    let w2 = thread::spawn(move || {
        let mut g = m2_w2.lock().unwrap();
        {
            let (lock, cvar) = &*ann_w2;
            let mut n = lock.lock().unwrap();
            *n += 1;
            cvar.notify_all();
        }
        while !*g {
            g = cv_w2.wait(g).unwrap();
        }
    });

    let m1_n = Arc::clone(&m1);
    let m2_n = Arc::clone(&m2);
    let cv_n = Arc::clone(&cv);
    let ann_n = Arc::clone(&announced);

    let notifier = thread::spawn(move || {
        {
            let (lock, cvar) = &*ann_n;
            let mut n = lock.lock().unwrap();
            while *n < 2 {
                n = cvar.wait(n).unwrap();
            }
        }
        let mut g1 = m1_n.lock().unwrap();
        let mut g2 = m2_n.lock().unwrap();
        *g1 = true;
        *g2 = true;
        cv_n.notify_all();
    });

    w1.join().unwrap();
    w2.join().unwrap();
    notifier.join().unwrap();

    println!("DONE done=1");
}
