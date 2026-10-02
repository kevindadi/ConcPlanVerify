use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn w1(s: Arc<Semaphore>) {
    let permit = s.acquire();
    let work1: i32 = 1;
    let _ = work1;
    permit.release();
}

fn w2(s: Arc<Semaphore>) {
    let permit = s.acquire();
    let work2: i32 = 1;
    let _ = work2;
    permit.release();
}

fn main() {
    let s = Semaphore::new(1);

    let h1 = {
        let s = Arc::clone(&s);
        thread::spawn(move || w1(s))
    };
    let h2 = {
        let s = Arc::clone(&s);
        thread::spawn(move || w2(s))
    };

    h1.join().expect("w1 thread panicked");
    h2.join().expect("w2 thread panicked");

    println!("DONE permits=1");
}
