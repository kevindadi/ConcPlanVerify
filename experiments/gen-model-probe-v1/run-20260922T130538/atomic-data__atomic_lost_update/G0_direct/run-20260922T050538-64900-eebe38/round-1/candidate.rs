use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

fn worker(counter: Arc<AtomicUsize>) {
    // Atomic read-modify-write retry loop: a failed CAS is retried with the
    // freshly observed value, so a competing update is never silently lost
    // and this worker's increment eventually takes effect.
    let mut current = counter.load(Ordering::Acquire);
    loop {
        match counter.compare_exchange_weak(
            current,
            current + 1,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => break,
            Err(actual) => current = actual,
        }
    }
}

fn main() {
    // Supervising task: shared counter starts at zero.
    let counter = Arc::new(AtomicUsize::new(0));

    // Launch two worker threads.
    let mut handles = Vec::with_capacity(2);
    for _ in 0..2 {
        let shared = Arc::clone(&counter);
        handles.push(thread::spawn(move || worker(shared)));
    }

    // Wait for both workers to finish; counter is now exactly two.
    for handle in handles {
        handle.join().expect("worker thread panicked");
    }

    println!("DONE done=1");
}
