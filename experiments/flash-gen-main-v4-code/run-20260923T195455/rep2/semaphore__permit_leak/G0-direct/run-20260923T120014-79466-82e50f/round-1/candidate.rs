use concir_sync::Semaphore;
use std::thread;

fn main() {
    let s = Semaphore::new(1);

    let s1 = s.clone();
    let w1 = thread::spawn(move || {
        let _permit = s1.acquire();
        // work while holding the permit
        // permit released on drop before finishing
    });

    let s2 = s.clone();
    let w2 = thread::spawn(move || {
        let _permit = s2.acquire();
        // work while holding the permit
        // permit released on drop before finishing
    });

    w1.join().unwrap();
    w2.join().unwrap();

    println!("DONE permits=1");
}
