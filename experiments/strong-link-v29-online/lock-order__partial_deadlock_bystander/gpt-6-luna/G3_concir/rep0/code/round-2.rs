use concir_sync::Semaphore;
use std::sync::{Arc, Mutex};
use std::thread;

struct AState {
    flag: bool,
}

fn a(a: Arc<Mutex<AState>>, b: Arc<Mutex<()>>, sa: Arc<Semaphore>, sb: Arc<Semaphore>) {
    drop(a.lock().unwrap());

    sa.acquire().release();

    let permit = sb.acquire();
    std::mem::forget(permit);

    let mut a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();
    a_guard.flag = true;
    drop(b_guard);
    drop(a_guard);
}

fn b(a: Arc<Mutex<AState>>, b: Arc<Mutex<()>>, sa: Arc<Semaphore>, sb: Arc<Semaphore>) {
    drop(b.lock().unwrap());

    sb.acquire().release();

    let permit = sa.acquire();
    std::mem::forget(permit);

    let mut a_guard = a.lock().unwrap();
    let b_guard = b.lock().unwrap();
    a_guard.flag = true;
    drop(b_guard);
    drop(a_guard);
}

fn bystander() {
    let mut i = true;
    loop {
        if i == true {
            i = true;
        } else {
            return;
        }
    }
}

fn main() {
    let a = Arc::new(Mutex::new(AState { flag: false }));
    let b = Arc::new(Mutex::new(()));
    let sa = Semaphore::new(0);
    let sb = Semaphore::new(0);

    let a_arg = Arc::clone(&a);
    let b_arg = Arc::clone(&b);
    let sa_arg = Arc::clone(&sa);
    let sb_arg = Arc::clone(&sb);
    let ha = thread::spawn(move || crate::a(a_arg, b_arg, sa_arg, sb_arg));

    let a_arg = Arc::clone(&a);
    let b_arg = Arc::clone(&b);
    let sa_arg = Arc::clone(&sa);
    let sb_arg = Arc::clone(&sb);
    let hb = thread::spawn(move || crate::b(a_arg, b_arg, sa_arg, sb_arg));

    let hby = thread::spawn(move || bystander());

    ha.join().unwrap();
    hb.join().unwrap();
    drop(hby);

    println!("DONE a=1 b=1");
}
