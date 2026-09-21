use std::sync::{Arc, Condvar, Mutex};
use std::thread;

fn main() {
    let m1 = Arc::new(Mutex::new(false));
    let m2 = Arc::new(Mutex::new(false));
    // shared mutex paired with the condvar
    let wait_m = Arc::new(Mutex::new(()));
    let cv = Arc::new(Condvar::new());
    let announced = Arc::new((Mutex::new(0usize), Condvar::new()));

    let m1_w = Arc::clone(&m1);
    let wait_w = Arc::clone(&wait_m);
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
        let mut wg = wait_w.lock().unwrap();
        while !*g {
            wg = cv_w.wait(wg).unwrap();
        }
    });
    ...
}
