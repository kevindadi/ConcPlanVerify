use concir_sync::Semaphore;
use std::sync::{Arc, Mutex};
use std::thread;

fn a<F>(
    a: Arc<Mutex<bool>>,
    b: Arc<Mutex<()>>,
    sa: Arc<Semaphore>,
    sb: Arc<Semaphore>,
    release_sa: F,
)
where
    F: FnOnce(),
{
    let guard = a.lock().unwrap();
    release_sa();
    drop(guard);

    let _permit = sb.acquire();

    let mut guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    *guard_a = true;
    drop(guard_b);
    drop(guard_a);
}

fn b<F>(
    a: Arc<Mutex<bool>>,
    b: Arc<Mutex<()>>,
    sa: Arc<Semaphore>,
    sb: Arc<Semaphore>,
    release_sb: F,
)
where
    F: FnOnce(),
{
    let guard = a.lock().unwrap();
    release_sb();
    drop(guard);

    let _permit = sa.acquire();

    let mut guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    *guard_a = true;
    drop(guard_b);
    drop(guard_a);
}

fn bystander() {
    let mut i: i32 = 0;
    loop {
        i = i.wrapping_add(1);
    }
}

fn main() {
    let a_lock = Arc::new(Mutex::new(false));
    let b_lock = Arc::new(Mutex::new(()));
    let sa = Semaphore::new(1);
    let sb = Semaphore::new(1);

    let permit_sa = sa.acquire();
    let permit_sb = sb.acquire();

    let ha = {
        let a = Arc::clone(&a_lock);
        let b = Arc::clone(&b_lock);
        let sa = Arc::clone(&sa);
        let sb = Arc::clone(&sb);
        thread::spawn(move || {
            a(
                a,
                b,
                sa,
                sb,
                move || permit_sa.release(),
            )
        })
    };

    let hb = {
        let a = Arc::clone(&a_lock);
        let b = Arc::clone(&b_lock);
        let sa = Arc::clone(&sa);
        let sb = Arc::clone(&sb);
        thread::spawn(move || {
            b(
                a,
                b,
                sa,
                sb,
                move || permit_sb.release(),
            )
        })
    };

    let _hby = thread::spawn(bystander);

    ha.join().unwrap();
    hb.join().unwrap();

    println!("DONE a=1 b=1");
}
