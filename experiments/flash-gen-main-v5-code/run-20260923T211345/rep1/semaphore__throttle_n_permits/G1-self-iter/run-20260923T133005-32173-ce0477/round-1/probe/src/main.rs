use concir_sync::Semaphore;
use std::thread;

fn main() {
    let s = Semaphore::new(2);

    let mut handles = Vec::new();

    for _ in 0..3 {
        let s = s.clone();
        handles.push(thread::spawn(move || {
            let permit = s.acquire();
            // perform work
            drop(permit);
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    println!("DONE done=1");
}
