use concir_sync::Semaphore;
use std::thread;

fn main() {
    let s = Semaphore::new(1);

    let s1 = s.clone();
    let w1 = thread::spawn(move || {
        let p1 = s1.acquire();
        // work
        p1.release();

        let p2 = s1.acquire();
        // work
        p2.release();
    });

    let s2 = s.clone();
    let w2 = thread::spawn(move || {
        let p1 = s2.acquire();
        // work
        p1.release();
    });

    w1.join().unwrap();
    w2.join().unwrap();

    println!("DONE done=1");
}
