use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn main() {
    let s = Semaphore::new(1);

    let s1 = Arc::clone(&s);
    let h1 = thread::spawn(move || {
        let _permit = s1.acquire();
    });

    let s2 = Arc::clone(&s);
    let h2 = thread::spawn(move || {
        let _permit = s2.acquire();
    });

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
}
