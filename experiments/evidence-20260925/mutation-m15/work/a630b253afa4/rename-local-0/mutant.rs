use concir_sync::Semaphore;
use std::thread;

fn main() {
    let s_kept = Semaphore::new(1);

    let s1 = s_kept.clone();
    let w1 = thread::spawn(move || {
        let permit = s1.acquire();
        permit.release();
    });

    let s2 = s_kept.clone();
    let w2 = thread::spawn(move || {
        let permit = s2.acquire();
        permit.release();
    });

    w1.join().unwrap();
    w2.join().unwrap();

    println!("DONE permits=1");
}
