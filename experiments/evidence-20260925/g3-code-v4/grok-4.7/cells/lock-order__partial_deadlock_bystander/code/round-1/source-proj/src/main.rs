use concir_sync::Semaphore;
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let run_a = a;
    let run_b = b;

    let a = Arc::new(Mutex::new(0i32));
    let b = Arc::new(Mutex::new(()));
    let sa = Semaphore::new(1);
    let sb = Semaphore::new(1);
    let sa_permit = sa.try_acquire().unwrap();
    let sb_permit = sb.try_acquire().unwrap();

    let by_h = thread::spawn(bystander);

    let h_a = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        let sb = Arc::clone(&sb);
        thread::spawn(move || run_a(a, b, sa_permit, sb))
    };
    let h_b = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        let sa = Arc::clone(&sa);
        thread::spawn(move || run_b(a, b, sb_permit, sa))
    };

    h_a.join().unwrap();
    h_b.join().unwrap();
    drop(by_h);

    let flag = *a.lock().unwrap();
    println!("DONE a={flag} b={flag}");
}

fn a(
    a: Arc<Mutex<i32>>,
    b: Arc<Mutex<()>>,
    sa_permit: concir_sync::Permit,
    sb: Arc<Semaphore>,
) {
    {
        let _guard_a = a.lock().unwrap();
        sa_permit.release();
    }
    let permit_sb = sb.acquire();
    {
        let mut guard_a = a.lock().unwrap();
        let guard_b = b.lock().unwrap();
        let flag = &mut *guard_a;
        *flag = 1;
        drop(guard_b);
        drop(guard_a);
    }
    std::mem::forget(permit_sb);
}

fn b(
    a: Arc<Mutex<i32>>,
    b: Arc<Mutex<()>>,
    sb_permit: concir_sync::Permit,
    sa: Arc<Semaphore>,
) {
    {
        let _guard_b = b.lock().unwrap();
        sb_permit.release();
    }
    let permit_sa = sa.acquire();
    {
        let mut guard_a = a.lock().unwrap();
        let guard_b = b.lock().unwrap();
        let flag = &mut *guard_a;
        *flag = 1;
        drop(guard_b);
        drop(guard_a);
    }
    std::mem::forget(permit_sa);
}

fn bystander() {
    loop {}
}
