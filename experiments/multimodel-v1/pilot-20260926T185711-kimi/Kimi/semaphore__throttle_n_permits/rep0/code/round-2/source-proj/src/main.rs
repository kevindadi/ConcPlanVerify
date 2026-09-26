use concir_sync::Semaphore;
use std::sync::Arc;

fn w1(s: Arc<Semaphore>) {
    let permit = s.acquire();
    let mut worked = 0;
    worked = 1;
    let _ = worked;
    permit.release();
}

fn w2(s: Arc<Semaphore>) {
    let permit = s.acquire();
    let mut worked = 0;
    worked = 1;
    let _ = worked;
    permit.release();
}

fn w3(s: Arc<Semaphore>) {
    let permit = s.acquire();
    let mut worked = 0;
    worked = 1;
    let _ = worked;
    permit.release();
}

fn supervisor() {
    let s = Semaphore::new(2);
    let h1 = std::thread::spawn({
        let s = s.clone();
        move || w1(s)
    });
    let h2 = std::thread::spawn({
        let s = s.clone();
        move || w2(s)
    });
    let h3 = std::thread::spawn({
        let s = s.clone();
        move || w3(s)
    });
    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();
}

fn main() {
    let h = std::thread::spawn(supervisor);
    h.join().unwrap();
    println!("DONE done=1");
}
