use concir_sync::Semaphore;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

fn compute() -> usize {
    let mut value = 1usize;
    for i in 1..=10 {
        value = value.wrapping_mul(i);
    }
    usize::from(value != 0)
}

fn w1(m: Arc<Semaphore>, acc: Arc<AtomicUsize>) {
    let permit = m.acquire();
    let amount = compute();
    acc.fetch_add(amount, Ordering::SeqCst);
    permit.release();
}

fn w2(m: Arc<Semaphore>, acc: Arc<AtomicUsize>) {
    let permit = m.acquire();
    let amount = compute();
    acc.fetch_add(amount, Ordering::SeqCst);
    permit.release();
}

fn main() {
    let m = Semaphore::new(1);
    let acc = Arc::new(AtomicUsize::new(0));

    let m1 = Arc::clone(&m);
    let acc1 = Arc::clone(&acc);
    let worker1 = thread::spawn(move || w1(m1, acc1));

    let m2 = Arc::clone(&m);
    let acc2 = Arc::clone(&acc);
    let worker2 = thread::spawn(move || w2(m2, acc2));

    worker1.join().unwrap();
    worker2.join().unwrap();

    let done = usize::from(acc.load(Ordering::SeqCst) == 2);
    println!("DONE done={done}");
}
