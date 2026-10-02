use concir_sync::{Permit, Semaphore};
use std::sync::{Arc, Mutex};
use std::thread;

fn a(
    a: Arc<Mutex<i32>>,
    b: Arc<Mutex<()>>,
    sa_seed: Permit,
    sb: Arc<Semaphore>,
) {
    {
        let _guard = a.lock().unwrap();
    }

    sa_seed.release();

    let _sb_permit = sb.acquire();
    let a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();
    *a_guard = 1;
    drop(b_guard);
    drop(a_guard);
}

fn b(
    a: Arc<Mutex<i32>>,
    b: Arc<Mutex<()>>,
    sa: Arc<Semaphore>,
    sb_seed: Permit,
) {
    let _sa_permit = sa.acquire();
    let a_guard = a.lock().unwrap();

    sb_seed.release();

    let b_guard = b.lock().unwrap();
    *a_guard = 1;
    drop(b_guard);
    drop(a_guard);
}

fn bystander() {
    loop {
        if 0 < 1 {
            continue;
        } else {
            return;
        }
    }
}

fn main() {
    let a = Arc::new(Mutex::new(0));
    let b = Arc::new(Mutex::new(()));
    let sa = Semaphore::new(1);
    let sb = Semaphore::new(1);

    let sa_seed = sa.acquire();
    let sb_seed = sb.acquire();

    let ha = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        let sb = Arc::clone(&sb);
        thread::spawn(move || a(a, b, sa_seed, sb))
    };

    let hb = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        let sa = Arc::clone(&sa);
        thread::spawn(move || b(a, b, sa, sb_seed))
    };

    let _hby = thread::spawn(move || bystander());

    ha.join().unwrap();
    hb.join().unwrap();

    let flag = *a.lock().unwrap();
    println!("DONE a={} b={}", flag, flag);
}
