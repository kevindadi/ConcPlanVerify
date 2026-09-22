use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Mutex;
use std::thread;

fn main() {
    let a = Mutex::new(());
    let b = Mutex::new(());
    let done = AtomicI32::new(0);

    let x1 = || {
        let guard_a = a.lock().unwrap();
        let guard_b = b.lock().unwrap();
        drop(guard_b);
        drop(guard_a);
    };

    let x2 = || {
        let guard_a = a.lock().unwrap();
        let guard_b = b.lock().unwrap();
        drop(guard_b);
        drop(guard_a);
    };

    let outer = || {
        thread::scope(|s| {
            s.spawn(&x1);
            s.spawn(&x2);
        });
    };

    thread::scope(|s| {
        s.spawn(&outer);
    });

    done.store(1, Ordering::SeqCst);
    println!("DONE done={}", done.load(Ordering::SeqCst));
}
