use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn t1(a: Arc<Semaphore>, b: Arc<Semaphore>) {
    let _pa = a.acquire();
    let _pb = b.acquire();
}

fn t2(a: Arc<Semaphore>, b: Arc<Semaphore>) {
    let _pa = a.acquire();
    let _pb = b.acquire();
}

fn t3(c: Arc<Semaphore>, d: Arc<Semaphore>) {
    let _pc = c.acquire();
    let _pd = d.acquire();
}

fn t4(c: Arc<Semaphore>, d: Arc<Semaphore>) {
    let _pc = c.acquire();
    let _pd = d.acquire();
}

fn main() {
    let a = Semaphore::new(1);
    let b = Semaphore::new(1);
    let c = Semaphore::new(1);
    let d = Semaphore::new(1);

    let h1 = thread::spawn({
        let a = a.clone();
        let b = b.clone();
        move || t1(a, b)
    });
    let h2 = thread::spawn({
        let a = a.clone();
        let b = b.clone();
        move || t2(a, b)
    });
    let h3 = thread::spawn({
        let c = c.clone();
        let d = d.clone();
        move || t3(c, d)
    });
    let h4 = thread::spawn({
        let c = c.clone();
        let d = d.clone();
        move || t4(c, d)
    });

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();
    h4.join().unwrap();

    println!("DONE done=1");
}
