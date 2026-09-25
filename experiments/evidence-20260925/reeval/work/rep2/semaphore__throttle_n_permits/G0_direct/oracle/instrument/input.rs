use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn main() {
    let s = Semaphore::new(2);

    let mut handles = Vec::new();

    for _ in 0..3 {
        let s = Arc::clone(&s);
        handles.push(thread::spawn(move || {
            let permit = s.acquire();
            // work
            permit.release();
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    println!("DONE done=1");
}
