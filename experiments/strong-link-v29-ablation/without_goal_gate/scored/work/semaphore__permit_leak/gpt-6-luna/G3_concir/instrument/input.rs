use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn w1(s: Arc<Semaphore>) {
    let permit = s.acquire();
    let mut work = 0;
    work = work + 1;
    permit.release();
}

fn w2(s: Arc<Semaphore>) {
    let permit = s.acquire();
    let mut work = 0;
    work = work + 1;
    permit.release();
}

fn main() {
    let s = Semaphore::new(1);

    let t1 = thread::spawn({
        let s = Arc::clone(&s);
        move || w1(s)
    });
    let t2 = thread::spawn({
        let s = Arc::clone(&s);
        move || w2(s)
    });

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE permits=1");
}
