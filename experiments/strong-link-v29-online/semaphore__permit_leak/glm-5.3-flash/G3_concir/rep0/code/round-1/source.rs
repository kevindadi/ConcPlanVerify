use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn w1(s: &Arc<Semaphore>) {
    let permit = s.acquire();
    permit.release();
}

fn w2(s: &Arc<Semaphore>) {
    let permit = s.acquire();
    permit.release();
}

fn main() {
    let s = Semaphore::new(1);

    let s1 = Arc::clone(&s);
    let t1 = thread::spawn(move || w1(&s1));

    let s2 = Arc::clone(&s);
    let t2 = thread::spawn(move || w2(&s2));

    t1.join().expect("w1 panicked");
    t2.join().expect("w2 panicked");

    println!("DONE permits=1");
}
