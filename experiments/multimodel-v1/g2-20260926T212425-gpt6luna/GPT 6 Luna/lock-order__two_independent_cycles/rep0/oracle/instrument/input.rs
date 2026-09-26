use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn t1(a: Arc<Semaphore>, b: Arc<Semaphore>) {
    let _a = a.acquire();
    let _b = b.acquire();
}

fn t2(a: Arc<Semaphore>, b: Arc<Semaphore>) {
    let _a = a.acquire();
    let _b = b.acquire();
}

fn t3(c: Arc<Semaphore>, d: Arc<Semaphore>) {
    let _c = c.acquire();
    let _d = d.acquire();
}

fn t4(c: Arc<Semaphore>, d: Arc<Semaphore>) {
    let _c = c.acquire();
    let _d = d.acquire();
}

fn main() {
    let a = Semaphore::new(1);
    let b = Semaphore::new(1);
    let c = Semaphore::new(1);
    let d = Semaphore::new(1);

    let h1 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        thread::spawn(move || t1(a, b))
    };
    let h2 = {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        thread::spawn(move || t2(a, b))
    };
    let h3 = {
        let c = Arc::clone(&c);
        let d = Arc::clone(&d);
        thread::spawn(move || t3(c, d))
    };
    let h4 = {
        let c = Arc::clone(&c);
        let d = Arc::clone(&d);
        thread::spawn(move || t4(c, d))
    };

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();
    h4.join().unwrap();

    println!("DONE done=1");
}
