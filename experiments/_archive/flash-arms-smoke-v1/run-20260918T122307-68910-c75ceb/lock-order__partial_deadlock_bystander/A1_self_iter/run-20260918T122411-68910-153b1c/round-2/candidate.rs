use std::sync::{Arc, Mutex, Semaphore};
use std::thread;
use std::time::Duration;

fn main() {
    let m1 = Arc::new(Mutex::new(()));
    let m2 = Arc::new(Mutex::new(()));
    let sem = Arc::new(Semaphore::new(0));

    let m1_a = Arc::clone(&m1);
    let m2_a = Arc::clone(&m2);
    let sem_a = Arc::clone(&sem);

    let a = thread::spawn(move || {
        // Wait for B's handshake before taking any lock, so A never
        // holds a mutex while blocked on the semaphore.
        sem_a.acquire().unwrap();
        let _g1 = m1_a.lock().unwrap();
        let _g2 = m2_a.lock().unwrap();
    });

    let m1_b = Arc::clone(&m1);
    let m2_b = Arc::clone(&m2);
    let sem_b = Arc::clone(&sem);

    let b = thread::spawn(move || {
        // Take both mutexes in the same global order (m1 then m2),
        // then signal A.
        let _g1 = m1_b.lock().unwrap();
        let _g2 = m2_b.lock().unwrap();
        sem_b.release(1);
    });

    let bystander = thread::spawn(|| loop {
        thread::sleep(Duration::from_millis(10));
    });

    a.join().unwrap();
    b.join().unwrap();
    bystander.join().unwrap();
}
