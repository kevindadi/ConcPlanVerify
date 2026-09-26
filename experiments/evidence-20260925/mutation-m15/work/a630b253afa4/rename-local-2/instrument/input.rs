use concir_sync::Semaphore;
use std::thread;

fn main() {
    let s = Semaphore::new(1);

    let s1 = s.clone();
    let w1_kept = thread::spawn(move || {
        let permit = s1.acquire();
        permit.release();
    });

    let s2 = s.clone();
    let w2 = thread::spawn(move || {
        let permit = s2.acquire();
        permit.release();
    });

    w1_kept.join().unwrap();
    w2.join().unwrap();

    println!("DONE permits=1");
}
