use concir_sync::Semaphore;
use std::thread;

fn main() {
    let s = Semaphore::new(2);

    let handles: Vec<_> = (0..3)
        .map(|_| {
            let s = s.clone();
            thread::spawn(move || {
                let _permit = s.acquire();
                // perform work
                drop(_permit);
            })
        })
        .collect();

    for h in handles {
        h.join().unwrap();
    }

    println!("DONE done=1");
}
