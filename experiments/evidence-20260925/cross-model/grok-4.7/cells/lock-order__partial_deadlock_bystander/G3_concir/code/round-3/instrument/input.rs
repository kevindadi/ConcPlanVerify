use concir_sync::Semaphore;
use std::sync::{Arc, Mutex};
use std::thread;

struct Protected {
    flag: bool,
}

fn a(a: Arc<Mutex<Protected>>, b: Arc<Mutex<()>>, release_sa: impl FnOnce(), sb: Arc<Semaphore>) {
    {
        let ga = a.lock().unwrap();
        release_sa();
        drop(ga);
    }
    std::mem::forget(sb.acquire());
    {
        let mut ga = a.lock().unwrap();
        let gb = b.lock().unwrap();
        ga.flag = true;
        drop(gb);
        drop(ga);
    }
}

fn b(a: Arc<Mutex<Protected>>, b: Arc<Mutex<()>>, sa: Arc<Semaphore>, release_sb: impl FnOnce()) {
    {
        let gb = b.lock().unwrap();
        release_sb();
        drop(gb);
    }
    std::mem::forget(sa.acquire());
    {
        let mut ga = a.lock().unwrap();
        let gb = b.lock().unwrap();
        ga.flag = true;
        drop(gb);
        drop(ga);
    }
}

fn bystander(a: Arc<Mutex<Protected>>) {
    loop {
        let ga = a.lock().unwrap();
        if ga.flag {
            drop(ga);
            return;
        }
        drop(ga);
    }
}

fn main() {
    let a = Arc::new(Mutex::new(Protected { flag: false }));
    let b = Arc::new(Mutex::new(()));
    let sa = Semaphore::new(1);
    let sb = Semaphore::new(1);
    let sa_permit = sa.try_acquire().unwrap();
    let sb_permit = sb.try_acquire().unwrap();

    let a_for_b = Arc::clone(&a);
    let a_for_by = Arc::clone(&a);
    let b_for_b = Arc::clone(&b);

    let ha = thread::spawn(move || {
        let release_sa = move || {
            sa_permit.release();
        };
        crate::a(a, b, release_sa, sb);
    });

    let hb = thread::spawn(move || {
        let release_sb = move || {
            sb_permit.release();
        };
        crate::b(a_for_b, b_for_b, sa, release_sb);
    });

    let hby = thread::spawn(move || {
        bystander(a_for_by);
    });

    ha.join().unwrap();
    hb.join().unwrap();
    hby.join().unwrap();
    println!("DONE a=1 b=1");
}
