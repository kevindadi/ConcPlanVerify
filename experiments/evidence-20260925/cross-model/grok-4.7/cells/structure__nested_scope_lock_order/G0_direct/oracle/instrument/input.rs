use concir_sync::Semaphore;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    let done = Arc::new(AtomicUsize::new(0));
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));
    let slots = Semaphore::new(2);

    let done_outer = Arc::clone(&done);
    let a_outer = Arc::clone(&a);
    let b_outer = Arc::clone(&b);

    let outer_handle = thread::spawn(move || {
        outer(done_outer, a_outer, b_outer, slots);
    });

    outer_handle.join().unwrap();
    println!("DONE done={}", done.load(Ordering::SeqCst));
}

fn outer(
    done: Arc<AtomicUsize>,
    a: Arc<Mutex<()>>,
    b: Arc<Mutex<()>>,
    slots: Arc<Semaphore>,
) {
    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let slots1 = Arc::clone(&slots);
    let x1_handle = thread::spawn(move || {
        let _permit = slots1.acquire();
        x1(a1, b1);
    });

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let slots2 = Arc::clone(&slots);
    let x2_handle = thread::spawn(move || {
        let _permit = slots2.acquire();
        x2(a2, b2);
    });

    x1_handle.join().unwrap();
    x2_handle.join().unwrap();
    done.store(1, Ordering::SeqCst);
}

fn x1(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    let _held = (&*guard_a, &*guard_b);
}

fn x2(a: Arc<Mutex<()>>, b: Arc<Mutex<()>>) {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();
    let _held = (&*guard_a, &*guard_b);
}
