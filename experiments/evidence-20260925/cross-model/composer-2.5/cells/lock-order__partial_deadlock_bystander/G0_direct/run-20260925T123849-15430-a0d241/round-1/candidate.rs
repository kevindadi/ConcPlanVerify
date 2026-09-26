use concir_sync::Semaphore;
use std::sync::{Arc, Mutex};
use std::thread;

fn a(
    a: Arc<Mutex<()>>,
    b: Arc<Mutex<()>>,
    sa: Arc<Semaphore>,
    sb: Arc<Semaphore>,
    flag: Arc<Mutex<(u8, u8)>>,
) {
    let permit = sb.acquire();
    let ga = a.lock().unwrap();
    permit.release();
    let _wait = sb.acquire();
    let gb = b.lock().unwrap();
    {
        let mut f = flag.lock().unwrap();
        f.0 = 1;
    }
    drop(gb);
    drop(ga);
}

fn b(
    a: Arc<Mutex<()>>,
    b: Arc<Mutex<()>>,
    sa: Arc<Semaphore>,
    sb: Arc<Semaphore>,
    flag: Arc<Mutex<(u8, u8)>>,
) {
    let permit = sa.acquire();
    let gb = b.lock().unwrap();
    permit.release();
    let _wait = sa.acquire();
    let ga = a.lock().unwrap();
    {
        let mut f = flag.lock().unwrap();
        f.1 = 1;
    }
    drop(ga);
    drop(gb);
}

fn bystander(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>, flag: Arc<Mutex<(u8, u8)>>) {
    loop {
        {
            let _f = flag.lock().unwrap();
        }
        if let Ok(g) = a.try_lock() {
            drop(g);
        }
        if let Ok(g) = b.try_lock() {
            drop(g);
        }
    }
}

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));
    let sa = Semaphore::new(1);
    let sb = Semaphore::new(1);
    let flag = Arc::new(Mutex::new((0u8, 0u8)));

    {
        let a_by = a.clone();
        let b_by = b.clone();
        let flag_by = flag.clone();
        thread::spawn(move || bystander(a_by, b_by, flag_by));
    }

    let ha = {
        let a_w = a.clone();
        let b_w = b.clone();
        let sa_w = sa.clone();
        let sb_w = sb.clone();
        let flag_w = flag.clone();
        thread::spawn(move || a(a_w, b_w, sa_w, sb_w, flag_w))
    };

    let hb = {
        let a_w = a.clone();
        let b_w = b.clone();
        let sa_w = sa.clone();
        let sb_w = sb.clone();
        let flag_w = flag.clone();
        thread::spawn(move || b(a_w, b_w, sa_w, sb_w, flag_w))
    };

    ha.join().unwrap();
    hb.join().unwrap();
    println!("DONE a=1 b=1");
}
