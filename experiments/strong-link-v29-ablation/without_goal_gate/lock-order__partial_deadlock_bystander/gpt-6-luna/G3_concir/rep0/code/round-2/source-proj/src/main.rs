use concir_sync::{Permit, Semaphore};
use std::sync::{Arc, Mutex};
use std::thread;

fn a(
    lock_a: Arc<Mutex<i32>>,
    lock_b: Arc<Mutex<()>>,
    sa_seed: Permit,
    sb: Arc<Semaphore>,
) {
    {
        let _guard = lock_a.lock().unwrap();
    }

    sa_seed.release();

    let _sb_permit = sb.acquire();
    let mut flag = lock_a.lock().unwrap();
    let guard_b = lock_b.lock().unwrap();
    *flag = 1;
    drop(guard_b);
    drop(flag);
}

fn b(
    lock_a: Arc<Mutex<i32>>,
    lock_b: Arc<Mutex<()>>,
    sa: Arc<Semaphore>,
    sb_seed: Permit,
) {
    let _sa_permit = sa.acquire();
    let mut flag = lock_a.lock().unwrap();

    sb_seed.release();

    let guard_b = lock_b.lock().unwrap();
    *flag = 1;
    drop(guard_b);
    drop(flag);
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
    let lock_a = Arc::new(Mutex::new(0));
    let lock_b = Arc::new(Mutex::new(()));
    let sa = Semaphore::new(1);
    let sb = Semaphore::new(1);

    let sa_seed = sa.acquire();
    let sb_seed = sb.acquire();

    let ha = {
        let lock_a = Arc::clone(&lock_a);
        let lock_b = Arc::clone(&lock_b);
        let sb = Arc::clone(&sb);
        thread::spawn(move || a(lock_a, lock_b, sa_seed, sb))
    };

    let hb = {
        let lock_a = Arc::clone(&lock_a);
        let lock_b = Arc::clone(&lock_b);
        let sa = Arc::clone(&sa);
        thread::spawn(move || b(lock_a, lock_b, sa, sb_seed))
    };

    let _hby = thread::spawn(move || bystander());

    ha.join().unwrap();
    hb.join().unwrap();

    let flag = *lock_a.lock().unwrap();
    println!("DONE a={} b={}", flag, flag);
}
