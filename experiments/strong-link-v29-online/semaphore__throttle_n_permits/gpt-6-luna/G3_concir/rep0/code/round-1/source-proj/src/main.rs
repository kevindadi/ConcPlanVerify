use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn w1(s: Arc<Semaphore>) {
    let permit = s.acquire();
    permit.release();
}

fn w2(s: Arc<Semaphore>) {
    let permit = s.acquire();
    permit.release();
}

fn w3(s: Arc<Semaphore>) {
    let permit = s.acquire();
    permit.release();
}

fn main() {
    let s = Semaphore::new(2);

    let t1 = thread::spawn({
        let s = Arc::clone(&s);
        move || w1(s)
    });
    let t2 = thread::spawn({
        let s = Arc::clone(&s);
        move || w2(s)
    });
    let t3 = thread::spawn({
        let s = Arc::clone(&s);
        move || w3(s)
    });

    t1.join().unwrap();
    t2.join().unwrap();
    t3.join().unwrap();

    println!("DONE done=1");
}
