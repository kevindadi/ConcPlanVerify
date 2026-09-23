use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn main() {
    let s = Semaphore::new(2);

    let mut handles = Vec::new();

    let s1 = Arc::clone(&s);
    handles.push(thread::spawn(move || {
        let _permit = s1.acquire();
        // work
    }));

    let s2 = Arc::clone(&s);
    handles.push(thread::spawn(move || {
        let _permit = s2.acquire();
        // work
    }));

    let s3 = Arc::clone(&s);
    handles.push(thread::spawn(move || {
        let _permit = s3.acquire();
        // work
    }));

    for h in handles {
        h.join().unwrap();
    }

    println!("DONE done=1");
}
