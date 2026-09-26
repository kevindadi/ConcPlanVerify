use concir_sync::Semaphore;
use std::sync::{Arc, Mutex};
use std::thread;

fn a(
    a: Arc<Mutex<()>>,
    b: Arc<Mutex<()>>,
    sa: Arc<Semaphore>,
    sb: Arc<Semaphore>,
    va: Arc<Mutex<u32>>,
) {
    let mut p_sa = sa.acquire();
    {
        let _ga = a.lock().unwrap();
        p_sa.release();
    }
    let _p_sb = sb.acquire();
    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
    *va.lock().unwrap() = 1;
}

fn b(
    a: Arc<Mutex<()>>,
    b: Arc<Mutex<()>>,
    sa: Arc<Semaphore>,
    sb: Arc<Semaphore>,
    vb: Arc<Mutex<u32>>,
) {
    let mut p_sb = sb.acquire();
    {
        let _gb = b.lock().unwrap();
        p_sb.release();
    }
    let _p_sa = sa.acquire();
    let _gb = b.lock().unwrap();
    let _ga = a.lock().unwrap();
    *vb.lock().unwrap() = 1;
}

fn bystander(flag: Arc<Mutex<u64>>) {
    loop {
        if let Ok(mut f) = flag.try_lock() {
            *f = f.wrapping_add(1);
        }
    }
}

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));
    let sa = Semaphore::new(1);
    let sb = Semaphore::new(1);
    let flag = Arc::new(Mutex::new(0u64));
    let va = Arc::new(Mutex::new(0u32));
    let vb = Arc::new(Mutex::new(0u32));

    let ha = {
        let a_lock = Arc::clone(&a);
        let b_lock = Arc::clone(&b);
        let sa = Arc::clone(&sa);
        let sb = Arc::clone(&sb);
        let va = Arc::clone(&va);
        thread::spawn(move || {
            crate::a(a_lock, b_lock, sa, sb, va);
        })
    };
    let hb = {
        let a_lock = Arc::clone(&a);
        let b_lock = Arc::clone(&b);
        let sa = Arc::clone(&sa);
        let sb = Arc::clone(&sb);
        let vb = Arc::clone(&vb);
        thread::spawn(move || {
            crate::b(a_lock, b_lock, sa, sb, vb);
        })
    };
    let _hbyst = {
        let flag = Arc::clone(&flag);
        thread::spawn(move || bystander(flag))
    };

    ha.join().unwrap();
    hb.join().unwrap();

    let a_count = *va.lock().unwrap();
    let b_count = *vb.lock().unwrap();
    println!("DONE a={} b={}", a_count, b_count);
}
