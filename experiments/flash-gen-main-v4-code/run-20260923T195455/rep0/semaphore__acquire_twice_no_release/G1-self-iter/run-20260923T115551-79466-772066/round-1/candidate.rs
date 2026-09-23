use concir_sync::Semaphore;
use std::thread;
use std::sync::Arc;

fn worker(name: &str, s: Arc<Semaphore>) {
    // Acquire the permit once
    let permit1 = s.acquire();
    // Do some work while holding the permit
    println!("{} working", name);
    // Acquire the permit a second time (nested acquisition)
    let permit2 = s.acquire();
    // Do more work
    println!("{} working again", name);
    // Release the second permit
    drop(permit2);
    // Release the first permit
    drop(permit1);
}

fn main() {
    let s = Semaphore::new(1);
    let s1 = Arc::clone(&s);
    let s2 = Arc::clone(&s);

    let w1 = thread::spawn(move || worker("w1", s1));
    let w2 = thread::spawn(move || worker("w2", s2));

    w1.join().unwrap();
    w2.join().unwrap();

    println!("DONE done=1");
}
