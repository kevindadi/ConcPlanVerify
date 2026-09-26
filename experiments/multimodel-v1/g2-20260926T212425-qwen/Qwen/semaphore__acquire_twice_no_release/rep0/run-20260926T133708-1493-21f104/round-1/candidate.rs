use concir_sync::Semaphore;
use std::sync::Arc;

fn main() {
    let s = Semaphore::new(1);

    let s1 = Arc::clone(&s);
    let w1 = std::thread::spawn(move || {
        // Acquire permit, do work, release permit.
        // The permit is released when the guard goes out of scope.
        let _permit = s1.acquire();
        // Work happens while holding the permit
        drop(_permit);
    });

    let s2 = Arc::clone(&s);
    let w2 = std::thread::spawn(move || {
        // Acquire permit, do work, release permit.
        let _permit = s2.acquire();
        // Work happens while holding the permit
        drop(_permit);
    });

    w1.join().unwrap();
    w2.join().unwrap();

    println!("DONE done=1");
}
