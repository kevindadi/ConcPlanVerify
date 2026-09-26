use concir_sync::Semaphore;
use std::sync::Arc;

fn w1(s: Arc<Semaphore>) {
    let mut work: i64 = 0;
    let p = s.acquire();
    work += 1;
    p.release();
    let p = s.acquire();
    work += 1;
    p.release();
    let _ = work;
}

fn w2(s: Arc<Semaphore>) {
    let mut work: i64 = 0;
    let p = s.acquire();
    work += 1;
    p.release();
    let p = s.acquire();
    work += 1;
    p.release();
    let _ = work;
}

fn main() {
    let s = Semaphore::new(1);

    let h1 = std::thread::spawn({
        let s = s.clone();
        move || w1(s)
    });
    let h2 = std::thread::spawn({
        let s = s.clone();
        move || w2(s)
    });

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
}
