use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn t1(a: Arc<Semaphore>, b: Arc<Semaphore>) {
    let _guard_a = a.acquire();
    let _guard_b = b.acquire();
}

fn t2(a: Arc<Semaphore>, b: Arc<Semaphore>) {
    let _guard_a = a.acquire();
    let _guard_b = b.acquire();
}

fn t3(c: Arc<Semaphore>, d: Arc<Semaphore>) {
    let _guard_c = c.acquire();
    let _guard_d = d.acquire();
}

fn t4(c: Arc<Semaphore>, d: Arc<Semaphore>) {
    let _guard_c = c.acquire();
    let _guard_d = d.acquire();
}

fn main() {
    let a = Semaphore::new(1);
    let b = Semaphore::new(1);
    let c = Semaphore::new(1);
    let d = Semaphore::new(1);

    let a1 = Arc::clone(&a);
    let b1 = Arc::clone(&b);
    let h1 = thread::spawn(move || t1(a1, b1));

    let a2 = Arc::clone(&a);
    let b2 = Arc::clone(&b);
    let h2 = thread::spawn(move || t2(a2, b2));

    let c3 = Arc::clone(&c);
    let d3 = Arc::clone(&d);
    let h3 = thread::spawn(move || t3(c3, d3));

    let c4 = Arc::clone(&c);
    let d4 = Arc::clone(&d);
    let h4 = thread::spawn(move || t4(c4, d4));

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();
    h4.join().unwrap();

    println!("DONE done=1");
}
