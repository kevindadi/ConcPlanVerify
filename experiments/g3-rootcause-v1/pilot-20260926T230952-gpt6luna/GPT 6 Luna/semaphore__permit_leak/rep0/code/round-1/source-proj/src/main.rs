use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn w1(s: Arc<Semaphore>) {
    let mut work = 0;
    let permit = s.acquire();
    work = 1;
    permit.release();
}

fn w2(s: Arc<Semaphore>) {
    let mut work = 0;
    let permit = s.acquire();
    work = 1;
    permit.release();
}

fn main() {
    let s = Semaphore::new(1);

    let s1 = Arc::clone(&s);
    let thread1 = thread::spawn(move || w1(s1));

    let s2 = Arc::clone(&s);
    let thread2 = thread::spawn(move || w2(s2));

    thread1.join().unwrap();
    thread2.join().unwrap();

    println!("DONE permits=1");
}
