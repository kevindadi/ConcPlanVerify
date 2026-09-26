use std::sync::{Arc, Mutex};
use std::thread;

use concir_sync::Semaphore;

struct Flag {
    a: i32,
    b: i32,
}

/// Worker a: first lock is `a`, second lock is `b`.
/// The permit handshake publishes "first lock taken" before either worker
/// acquires its second lock. The first lock is dropped before the second
/// acquisition, then both locks are taken in the global order a -> b.
fn a<P: Send + 'static>(
    a: Arc<Mutex<()>>,
    b: Arc<Mutex<()>>,
    sb: Arc<Semaphore>,
    flag: Arc<Mutex<Flag>>,
    permit_a: P,
    publish_a: impl FnOnce(P),
) {
    let first = a.lock().unwrap();
    publish_a(permit_a);
    let peer = sb.acquire();
    drop(first);
    peer.release();

    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
    flag.lock().unwrap().a = 1;
}

/// Worker b: first lock is `b`, second lock is `a`.
fn b<P: Send + 'static>(
    a: Arc<Mutex<()>>,
    b: Arc<Mutex<()>>,
    sa: Arc<Semaphore>,
    flag: Arc<Mutex<Flag>>,
    permit_b: P,
    publish_b: impl FnOnce(P),
) {
    let first = b.lock().unwrap();
    publish_b(permit_b);
    let peer = sa.acquire();
    drop(first);
    peer.release();

    let _ga = a.lock().unwrap();
    let _gb = b.lock().unwrap();
    flag.lock().unwrap().b = 1;
}

/// Independent bystander: keeps taking its own lock and counting forever.
/// It never touches the workers' locks or permits.
fn bystander() {
    let gate = Mutex::new(0usize);
    loop {
        let mut step = gate.lock().unwrap();
        *step = step.wrapping_add(1);
    }
}

fn main() {
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));
    let sa = Semaphore::new(1);
    let sb = Semaphore::new(1);
    let flag = Arc::new(Mutex::new(Flag { a: 0, b: 0 }));

    // Hold the initial permits so each semaphore starts empty. Releasing a
    // permit after the first lock is the handshake signal the peer waits for.
    let permit_a = sa.acquire();
    let permit_b = sb.acquire();

    let bystander_handle = thread::spawn(bystander);

    let a_for_a = Arc::clone(&a);
    let b_for_a = Arc::clone(&b);
    let sb_for_a = Arc::clone(&sb);
    let flag_for_a = Arc::clone(&flag);
    let worker_a = thread::spawn(move || {
        crate::a(a_for_a, b_for_a, sb_for_a, flag_for_a, permit_a, |permit| {
            permit.release();
        });
    });

    let a_for_b = Arc::clone(&a);
    let b_for_b = Arc::clone(&b);
    let sa_for_b = Arc::clone(&sa);
    let flag_for_b = Arc::clone(&flag);
    let worker_b = thread::spawn(move || {
        crate::b(a_for_b, b_for_b, sa_for_b, flag_for_b, permit_b, |permit| {
            permit.release();
        });
    });

    worker_a.join().unwrap();
    worker_b.join().unwrap();

    let done = flag.lock().unwrap();
    println!("DONE a={} b={}", done.a, done.b);
    drop(done);

    // The bystander does not finish on its own. Detach it and let process
    // exit satisfy termination after both workers have finished.
    drop(bystander_handle);
}
