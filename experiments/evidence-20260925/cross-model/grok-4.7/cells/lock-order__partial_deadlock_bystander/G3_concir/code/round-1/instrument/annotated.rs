mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use concir_sync::Semaphore;
use std::sync::{Arc};
use std::thread;

struct Shared {
    flag: bool,
}

fn a(
    a: Arc<Mutex<Shared>>,
    b: Arc<Mutex<()>>,
    release_sa: Box<dyn FnOnce() + Send>,
    sb: Arc<Semaphore>,
) {
    let guard_a = a.lock().unwrap();
    release_sa();
    drop(guard_a);

    let _permit_sb = sb.acquire();

    let mut guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    guard_a.flag = true;
    drop(guard_b);
    drop(guard_a);
}

fn b(
    a: Arc<Mutex<Shared>>,
    b: Arc<Mutex<()>>,
    release_sb: Box<dyn FnOnce() + Send>,
    sa: Arc<Semaphore>,
) {
    let guard_b = b.lock().unwrap();
    release_sb();
    drop(guard_b);

    let _permit_sa = sa.acquire();

    let mut guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    guard_a.flag = true;
    drop(guard_b);
    drop(guard_a);
}

fn bystander(a: Arc<Mutex<Shared>>) {
    loop {
        let guard_a = a.lock().unwrap();
        if guard_a.flag == true {
            drop(guard_a);
            return;
        } else {
            drop(guard_a);
        }
    }
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0", Shared { flag: false }));
    let b = Arc::new(Mutex::new_named("b_mutex0", ()));
    let sa = Semaphore::new_named("sa_semaphore0", 1);
    let sb = Semaphore::new_named("sb_semaphore0", 1);
    let permit_sa = sa.acquire();
    let permit_sb = sb.acquire();

    let release_sa: Box<dyn FnOnce() + Send> = Box::new(move || {
        permit_sa.release();
    });
    let release_sb: Box<dyn FnOnce() + Send> = Box::new(move || {
        permit_sb.release();
    });

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let sb1 = Arc::clone(&sb);
    let ha = cir_trace::spawn("a", move || {
        crate::a(a1, b1, release_sa, sb1);
    });

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let sa2 = Arc::clone(&sa);
    let hb = cir_trace::spawn("b", move || {
        crate::b(a2, b2, release_sb, sa2);
    });

    let a3 = Arc::clone(&a);
    let hby = cir_trace::spawn("bystander", move || {
        crate::bystander(a3);
    });

    ha.join().unwrap();
    hb.join().unwrap();
    hby.join().unwrap();
    println!("DONE a=1 b=1");
 cir_trace::finish();}
