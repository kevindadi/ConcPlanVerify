use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn main() {
    let s = Semaphore::new(2);
    let s1 = Arc::clone(&s);
    let s2 = Arc::clone(&s);
    let s3 = Arc::clone(&s);

    let w1 = thread::spawn(move || {
        let _permit = s1.acquire();
        // work performed while holding permit
    });

    let w2 = thread::spawn(move || {
        let _permit = s2.acquire();
        // work performed while holding permit
    });

    let w3 = thread::spawn(move || {
        let _permit = s3.acquire();
        // work performed while holding permit
    });

    w1.join().unwrap();
    w2.join().unwrap();
    w3.join().unwrap();

    println!("DONE done=1");
}
