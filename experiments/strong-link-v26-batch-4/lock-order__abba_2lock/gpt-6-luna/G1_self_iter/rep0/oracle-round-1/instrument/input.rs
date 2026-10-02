use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::thread;

fn t1(a: &Mutex<()>, b: &Mutex<()>, result: &AtomicUsize) {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();

    result.store(1, Ordering::SeqCst);

    drop(guard_b);
    drop(guard_a);
}

fn t2(a: &Mutex<()>, b: &Mutex<()>, result: &AtomicUsize) {
    let guard_a = a.lock().unwrap();
    let guard_b = b.lock().unwrap();

    result.store(1, Ordering::SeqCst);

    drop(guard_b);
    drop(guard_a);
}

fn main() {
    let a = Mutex::new(());
    let b = Mutex::new(());
    let result_t1 = AtomicUsize::new(0);
    let result_t2 = AtomicUsize::new(0);

    thread::scope(|scope| {
        let worker_t1 = scope.spawn(|| t1(&a, &b, &result_t1));
        let worker_t2 = scope.spawn(|| t2(&a, &b, &result_t2));

        worker_t1.join().unwrap();
        worker_t2.join().unwrap();
    });

    println!(
        "DONE t1={} t2={}",
        result_t1.load(Ordering::SeqCst),
        result_t2.load(Ordering::SeqCst)
    );
}
