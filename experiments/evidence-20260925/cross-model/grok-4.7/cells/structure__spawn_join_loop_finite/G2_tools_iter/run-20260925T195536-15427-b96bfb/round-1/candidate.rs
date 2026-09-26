use concir_sync::Semaphore;
use std::thread;

fn worker() {}

fn main() {
    let _sem = Semaphore::new(1);
    for _ in 0..2 {
        let handle = thread::spawn(worker);
        handle.join().unwrap();
    }
    let done = 1;
    println!("DONE done={done}");
}
