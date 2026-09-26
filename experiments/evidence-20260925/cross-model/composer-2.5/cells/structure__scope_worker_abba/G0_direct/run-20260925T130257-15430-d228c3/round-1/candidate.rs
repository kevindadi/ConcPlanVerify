use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn w1(a: Arc<Semaphore>, b: Arc<Semaphore>) {
    let _guard_a = a.acquire();
    let _guard_b = b.acquire();
}

fn w2(a: Arc<Semaphore>, b: Arc<Semaphore>) {
    let _guard_a = a.acquire();
    let _guard_b = b.acquire();
}

fn main() {
    let a = Semaphore::new(1);
    let b = Semaphore::new(1);

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);

    let handle_w1 = thread::spawn(move || w1(a1, b1));
    let handle_w2 = thread::spawn(move || w2(a2, b2));

    handle_w1.join().unwrap();
    handle_w2.join().unwrap();

    println!("DONE done=1");
}
