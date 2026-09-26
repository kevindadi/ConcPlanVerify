use std::sync::{Arc, Mutex};
use std::thread;

use concir_sync::Semaphore;

struct AState {
    flag: bool,
}

fn make_release(sem: &'static Arc<Semaphore>) -> impl FnOnce() + Send + 'static {
    let sem: &'static Semaphore = sem.as_ref();
    let permit = sem.acquire();
    move || {
        permit.release();
    }
}

fn a(
    a: Arc<Mutex<AState>>,
    b: Arc<Mutex<()>>,
    release_sa: impl FnOnce() + Send + 'static,
    sb: Arc<Semaphore>,
) {
    {
        let guard_a = a.lock().unwrap();
        release_sa();
        drop(guard_a);
    }
    std::mem::forget(sb.acquire());
    {
        let mut guard_a = a.lock().unwrap();
        let guard_b = b.lock().unwrap();
        guard_a.flag = true;
        drop(guard_b);
        drop(guard_a);
    }
}

fn b(
    a: Arc<Mutex<AState>>,
    b: Arc<Mutex<()>>,
    release_sb: impl FnOnce() + Send + 'static,
    sa: Arc<Semaphore>,
) {
    {
        let guard_b = b.lock().unwrap();
        release_sb();
        drop(guard_b);
    }
    std::mem::forget(sa.acquire());
    {
        let mut guard_a = a.lock().unwrap();
        let guard_b = b.lock().unwrap();
        guard_a.flag = true;
        drop(guard_b);
        drop(guard_a);
    }
}

fn bystander(a: Arc<Mutex<AState>>) {
    loop {
        let guard_a = a.lock().unwrap();
        if guard_a.flag {
            drop(guard_a);
            return;
        }
        drop(guard_a);
    }
}

fn main() {
    let a = Arc::new(Mutex::new(AState { flag: false }));
    let b = Arc::new(Mutex::new(()));
    let sa: &'static Arc<Semaphore> = Box::leak(Box::new(Semaphore::new(1)));
    let sb: &'static Arc<Semaphore> = Box::leak(Box::new(Semaphore::new(1)));
    let release_sa = make_release(sa);
    let release_sb = make_release(sb);

    let a_for_a = Arc::clone(&a);
    let b_for_a = Arc::clone(&b);
    let sb_for_a = Arc::clone(sb);
    let ha = thread::spawn(move || {
        crate::a(a_for_a, b_for_a, release_sa, sb_for_a);
    });

    let a_for_b = Arc::clone(&a);
    let b_for_b = Arc::clone(&b);
    let sa_for_b = Arc::clone(sa);
    let hb = thread::spawn(move || {
        crate::b(a_for_b, b_for_b, release_sb, sa_for_b);
    });

    let a_for_by = Arc::clone(&a);
    let hby = thread::spawn(move || {
        crate::bystander(a_for_by);
    });

    ha.join().unwrap();
    hb.join().unwrap();
    hby.join().unwrap();
    println!("DONE a=1 b=1");
}
