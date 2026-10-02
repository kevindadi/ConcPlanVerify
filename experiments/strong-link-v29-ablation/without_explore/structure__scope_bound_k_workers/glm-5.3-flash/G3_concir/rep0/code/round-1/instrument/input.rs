use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn w1(s: &Arc<Semaphore>) {
    let _permit = s.acquire();
    // work while holding the single permit
    drop(_permit);
}

fn w2(s: &Arc<Semaphore>) {
    let _permit = s.acquire();
    // work while holding the single permit
    drop(_permit);
}

fn w3(s: &Arc<Semaphore>) {
    let _permit = s.acquire();
    // work while holding the single permit
    drop(_permit);
}

fn main() {
    let s = Semaphore::new(1);

    let s1 = Arc::clone(&s);
    let t1 = thread::spawn(move || w1(&s1));
    let s2 = Arc::clone(&s);
    let t2 = thread::spawn(move || w2(&s2));
    let s3 = Arc::clone(&s);
    let t3 = thread::spawn(move || w3(&s3));

    t1.join().expect("w1 panicked");
    t2.join().expect("w2 panicked");
    t3.join().expect("w3 panicked");

    println!("DONE done=1");
}
