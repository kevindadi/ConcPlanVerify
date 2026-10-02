use concir_sync::Semaphore;
use std::sync::{Arc, Mutex};
use std::thread;

fn a(
    a: Arc<Mutex<i32>>,
    b: Arc<Mutex<()>>,
    sa: Arc<Semaphore>,
    sb: Arc<Semaphore>,
) {
    let sa_permit = sa.acquire();
    let mut a_guard = a.lock().unwrap();
    sa_permit.release();
    drop(a_guard);

    let sb_permit = sb.acquire();
    let mut a_guard = a.lock().unwrap();
    let _b_guard = b.lock().unwrap();
    *a_guard += 1;
    drop(_b_guard);
    drop(a_guard);
    drop(sb_permit);
}

fn b(
    a: Arc<Mutex<i32>>,
    b: Arc<Mutex<()>>,
    sa: Arc<Semaphore>,
    sb: Arc<Semaphore>,
) {
    let sb_permit = sb.acquire();
    let _sa_permit = sa.acquire();
    let a_guard = a.lock().unwrap();
    sb_permit.release();
    drop(a_guard);

    let mut a_guard = a.lock().unwrap();
    let _b_guard = b.lock().unwrap();
    *a_guard += 1;
    drop(_b_guard);
    drop(a_guard);
}

fn bystander() {
    let mut i = 0;
    loop {
        if i < 0 {
            return;
        }
        i = 1;
        i = 0;
    }
}

fn main() {
    let a_lock = Arc::new(Mutex::new(0));
    let b_lock = Arc::new(Mutex::new(()));
    let sa = Semaphore::new(1);
    let sb = Semaphore::new(1);

    let ha = {
        let a = Arc::clone(&a_lock);
        let b = Arc::clone(&b_lock);
        let sa = Arc::clone(&sa);
        let sb = Arc::clone(&sb);
        thread::spawn(move || a(a, b, sa, sb))
    };
    let hb = {
        let a = Arc::clone(&a_lock);
        let b = Arc::clone(&b_lock);
        let sa = Arc::clone(&sa);
        let sb = Arc::clone(&sb);
        thread::spawn(move || b(a, b, sa, sb))
    };
    let _hby = thread::spawn(move || bystander());

    ha.join().unwrap();
    hb.join().unwrap();
    println!("DONE a=1 b=1");
}
