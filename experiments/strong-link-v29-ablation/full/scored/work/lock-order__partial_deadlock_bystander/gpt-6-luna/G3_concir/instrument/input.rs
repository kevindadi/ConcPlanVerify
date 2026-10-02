use concir_sync::Semaphore;
use std::sync::{Arc, Mutex};
use std::thread;

struct AState {
    flag: bool,
}

fn a<F>(
    a_lock: Arc<Mutex<AState>>,
    b_lock: Arc<Mutex<()>>,
    sb: Arc<Semaphore>,
    release_sa: F,
)
where
    F: FnOnce(),
{
    let guard = a_lock.lock().unwrap();
    release_sa();
    drop(guard);

    let _permit = sb.acquire();

    let mut a_guard = a_lock.lock().unwrap();
    let _b_guard = b_lock.lock().unwrap();
    a_guard.flag = true;
    a_guard.flag = false;
    drop(_b_guard);
    drop(a_guard);
}

fn b<F>(
    a_lock: Arc<Mutex<AState>>,
    b_lock: Arc<Mutex<()>>,
    sa: Arc<Semaphore>,
    release_sb: F,
)
where
    F: FnOnce(),
{
    let guard = b_lock.lock().unwrap();
    release_sb();
    drop(guard);

    let _permit = sa.acquire();

    let mut a_guard = a_lock.lock().unwrap();
    let _b_guard = b_lock.lock().unwrap();
    a_guard.flag = true;
    a_guard.flag = false;
    drop(_b_guard);
    drop(a_guard);
}

fn bystander() {
    let mut phase = 0;
    loop {
        if phase == 0 {
            phase = 1;
        } else {
            phase = 0;
        }
    }
}

fn main() {
    let a_lock = Arc::new(Mutex::new(AState { flag: false }));
    let b_lock = Arc::new(Mutex::new(()));
    let sa = Semaphore::new(1);
    let sb = Semaphore::new(1);

    let permit_sa = sa.try_acquire().unwrap();
    let permit_sb = sb.try_acquire().unwrap();

    let ha = {
        let a_lock = Arc::clone(&a_lock);
        let b_lock = Arc::clone(&b_lock);
        let sb = Arc::clone(&sb);
        thread::spawn(move || a(a_lock, b_lock, sb, move || permit_sa.release()))
    };

    let hb = {
        let a_lock = Arc::clone(&a_lock);
        let b_lock = Arc::clone(&b_lock);
        let sa = Arc::clone(&sa);
        thread::spawn(move || b(a_lock, b_lock, sa, move || permit_sb.release()))
    };

    let hby = thread::spawn(move || bystander());

    ha.join().unwrap();
    hb.join().unwrap();
    drop(hby);

    println!("DONE a=1 b=1");
}
