mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;

use concir_sync::Semaphore;

struct Flag {
    a: AtomicUsize,
    b: AtomicUsize,
    stop: AtomicBool,
}

fn a(
    a: Arc<Mutex<()>>,
    b: Arc<Mutex<()>>,
    sa: Arc<Semaphore>,
    sb: Arc<Semaphore>,
    flag: Arc<Flag>,
    ready: Arc<Barrier>,
) {
    // Own sa's token before either side signals, so the later release is the handshake.
    let permit = sa.acquire();
    let _ = ready.wait();

    let first = a.lock().unwrap();
    permit.release();
    let peer = sb.acquire();
    drop(peer);
    drop(first);

    let hold_a = a.lock().unwrap();
    let hold_b = b.lock().unwrap();
    flag.a.store(1, Ordering::SeqCst);
    drop(hold_b);
    drop(hold_a);
}

fn b(
    a: Arc<Mutex<()>>,
    b: Arc<Mutex<()>>,
    sa: Arc<Semaphore>,
    sb: Arc<Semaphore>,
    flag: Arc<Flag>,
    ready: Arc<Barrier>,
) {
    let permit = sb.acquire();
    let _ = ready.wait();

    let first = b.lock().unwrap();
    permit.release();
    let peer = sa.acquire();
    drop(peer);
    drop(first);

    let hold_a = a.lock().unwrap();
    let hold_b = b.lock().unwrap();
    flag.b.store(1, Ordering::SeqCst);
    drop(hold_b);
    drop(hold_a);
}

fn bystander(flag: Arc<Flag>) {
    while !flag.stop.load(Ordering::SeqCst) {
        let _ = flag.a.load(Ordering::Relaxed);
        let _ = flag.b.load(Ordering::Relaxed);
        std::hint::spin_loop();
    }
}

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0", ()));
    let sa = Semaphore::new_named("sa_semaphore0", 1);
    let sb = Semaphore::new_named("sb_semaphore0", 1);
    let flag = Arc::new(Flag {
        a: AtomicUsize::new(0),
        b: AtomicUsize::new(0),
        stop: AtomicBool::new(false),
    });
    let ready = Arc::new(Barrier::new(2));

    let handle_a = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        let sa = Arc::clone(&sa);
        let sb = Arc::clone(&sb);
        let flag = Arc::clone(&flag);
        let ready = Arc::clone(&ready);
        cir_trace::spawn("a", move || crate::a(a, b, sa, sb, flag, ready))
    };

    let handle_b = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        let sa = Arc::clone(&sa);
        let sb = Arc::clone(&sb);
        let flag = Arc::clone(&flag);
        let ready = Arc::clone(&ready);
        cir_trace::spawn("b", move || crate::b(a, b, sa, sb, flag, ready))
    };

    let handle_bystander = {
        let flag = Arc::clone(&flag);
        cir_trace::spawn("bystander", move || bystander(flag))
    };

    handle_a.join().unwrap();
    handle_b.join().unwrap();

    println!(
        "DONE a={} b={}",
        flag.a.load(Ordering::SeqCst),
        flag.b.load(Ordering::SeqCst)
    );

    flag.stop.store(true, Ordering::SeqCst);
    handle_bystander.join().unwrap();
 cir_trace::finish();}
