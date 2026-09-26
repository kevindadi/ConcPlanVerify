use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn w1(s: Arc<Semaphore>) {
    let mut work = 0;
    let permit = s.acquire();
    work = 1;
    permit.release();

    let permit = s.acquire();
    work = 2;
    permit.release();

    let _ = work;
}

fn w2(s: Arc<Semaphore>) {
    let mut work = 0;
    let permit = s.acquire();
    work = 1;
    permit.release();

    let permit = s.acquire();
    work = 2;
    permit.release();

    let _ = work;
}

fn w3(s: Arc<Semaphore>) {
    let mut work = 0;
    let permit = s.acquire();
    work = 1;
    permit.release();

    let permit = s.acquire();
    work = 2;
    permit.release();

    let _ = work;
}

fn main() {
    let s = Semaphore::new(1);

    let s1 = Arc::clone(&s);
    let t1 = thread::spawn(move || w1(s1));

    let s2 = Arc::clone(&s);
    let t2 = thread::spawn(move || w2(s2));

    let s3 = Arc::clone(&s);
    let t3 = thread::spawn(move || w3(s3));

    t1.join().unwrap();
    t2.join().unwrap();
    t3.join().unwrap();

    println!("DONE done=1");
}
