use std::thread;
use concir_sync::Semaphore;

fn helper() {
    let sem = Semaphore::new(1);
    let permit = sem.acquire();
    permit.release();
}

fn main() {
    let handle = thread::spawn(helper);
    handle.join().unwrap();

    let handle = thread::spawn(helper);
    handle.join().unwrap();

    println!("DONE done=1");
}
