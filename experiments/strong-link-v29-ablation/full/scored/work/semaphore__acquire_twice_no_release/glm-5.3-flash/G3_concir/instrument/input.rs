use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn w1(s: &Arc<Semaphore>) {
    let mut i: i32 = 0;
    while i < 2 {
        let permit = s.acquire();
        i = i + 1;
        permit.release();
    }
}

fn w2(s: &Arc<Semaphore>) {
    let mut i: i32 = 0;
    while i < 2 {
        let permit = s.acquire();
        i = i + 1;
        permit.release();
    }
}

fn main() {
    let s = Semaphore::new(1);
    let h1 = {
        let s = Arc::clone(&s);
        thread::spawn(move || w1(&s))
    };
    let h2 = {
        let s = Arc::clone(&s);
        thread::spawn(move || w2(&s))
    };
    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");
    println!("DONE done=1");
}
