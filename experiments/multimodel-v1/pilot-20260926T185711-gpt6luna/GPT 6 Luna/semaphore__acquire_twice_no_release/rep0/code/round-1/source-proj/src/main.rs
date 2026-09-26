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

fn main() {
    let s = Semaphore::new(1);

    let s1 = Arc::clone(&s);
    let h1 = thread::spawn(move || w1(s1));

    let s2 = Arc::clone(&s);
    let h2 = thread::spawn(move || w2(s2));

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
}
