use concir_sync::Semaphore;
use std::sync::Arc;

fn main() {
    let s = Semaphore::new(1);

    let s1 = Arc::clone(&s);
    let w1_handle = std::thread::spawn(move || {
        let permit = s1.acquire();
        // Perform work while holding the permit
        drop(permit);
    });

    let s2 = Arc::clone(&s);
    let w2_handle = std::thread::spawn(move || {
        let permit = s2.acquire();
        // Perform work while holding the permit
        drop(permit);
    });

    w1_handle.join().unwrap();
    w2_handle.join().unwrap();

    println!("DONE permits=1");
}
