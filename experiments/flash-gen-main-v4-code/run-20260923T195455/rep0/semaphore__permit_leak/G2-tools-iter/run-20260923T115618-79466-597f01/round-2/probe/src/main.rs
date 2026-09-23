use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn main() {
    let s: Arc<Semaphore> = Semaphore::new(1);

    let s1 = Arc::clone(&s);
    let w1 = thread::spawn(move || {
        let permit = s1.acquire();
        // perform work while holding the permit
        drop(permit);
    });

    let s2 = Arc::clone(&s);
    let w2 = thread::spawn(move || {
        let permit = s2.acquire();
        // perform work while holding the permit
        drop(permit);
    });

    w1.join().unwrap();
    w2.join().unwrap();

    println!("DONE permits=1");
}
