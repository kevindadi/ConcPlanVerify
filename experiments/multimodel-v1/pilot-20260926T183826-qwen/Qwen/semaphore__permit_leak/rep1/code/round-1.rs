use concir_sync::Semaphore;
use std::sync::Arc;

fn main() {
    let s = Semaphore::new(1);
    let s1 = Arc::clone(&s);
    let s2 = Arc::clone(&s);

    let w1 = std::thread::spawn(move || {
        let _permit = s1.acquire();
    });

    let w2 = std::thread::spawn(move || {
        let _permit = s2.acquire();
    });

    w1.join().unwrap();
    w2.join().unwrap();

    println!("DONE permits=1");
}
