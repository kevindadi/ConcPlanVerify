use concir_sync::Semaphore;
use std::thread;

fn main() {
    let a = Semaphore::new(1);
    let b = Semaphore::new(1);

    let a1 = a.clone();
    let b1 = b.clone();
    let w1 = thread::spawn(move || {
        let _hold_a = a1.acquire();
        let _hold_b = b1.acquire();
    });

    let a2 = a.clone();
    let b2 = b.clone();
    let w2 = thread::spawn(move || {
        let _hold_a = a2.acquire();
        let _hold_b = b2.acquire();
    });

    w1.join().unwrap();
    w2.join().unwrap();
    println!("DONE done=1");
}
