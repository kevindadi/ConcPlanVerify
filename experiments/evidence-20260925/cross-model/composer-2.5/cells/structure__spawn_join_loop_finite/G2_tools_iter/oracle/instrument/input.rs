use concir_sync::Semaphore;
use std::thread;

fn worker() {}

fn main() {
    let sem = Semaphore::new(1);
    for _ in 0..2 {
        let _permit = sem.acquire();
        let handle = thread::spawn(worker);
        handle.join().unwrap();
    }
    println!("DONE done=1");
}
