use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn w1(a: Arc<Semaphore>, b: Arc<Semaphore>) {
    let _perm_a = a.acquire();
    let _perm_b = b.acquire();
}

fn w2(a: Arc<Semaphore>, b: Arc<Semaphore>) {
    let _perm_a = a.acquire();
    let _perm_b = b.acquire();
}

fn main() {
    let a = Semaphore::new(1);
    let b = Semaphore::new(1);

    let a_w2 = Arc::clone(&a);
    let b_w2 = Arc::clone(&b);

    let h1 = thread::spawn(move || w1(a, b));
    let h2 = thread::spawn(move || w2(a_w2, b_w2));

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
}
