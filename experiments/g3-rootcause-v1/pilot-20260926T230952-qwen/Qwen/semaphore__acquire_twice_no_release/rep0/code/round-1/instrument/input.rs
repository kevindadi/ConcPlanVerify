use concir_sync::Semaphore;
use std::sync::Arc;

fn w1(s: Arc<Semaphore>) {
    let permit = s.acquire();
    // Perform work while holding the permit
    drop(permit);
}

fn w2(s: Arc<Semaphore>) {
    let permit = s.acquire();
    // Perform work while holding the permit
    drop(permit);
}

fn main() {
    let s = Semaphore::new(1);

    let s1 = Arc::clone(&s);
    let handle1 = std::thread::spawn(move || {
        w1(s1);
    });

    let s2 = Arc::clone(&s);
    let handle2 = std::thread::spawn(move || {
        w2(s2);
    });

    handle1.join().unwrap();
    handle2.join().unwrap();

    println!("DONE done=1");
}
