use std::sync::Arc;
use std::thread;

use concir_sync::Semaphore;

fn w1(s: Arc<Semaphore>) {
    let permit = s.acquire();
    let mut acc = 0u32;
    acc = acc.wrapping_add(1);
    std::hint::black_box(acc);
    permit.release();
}

fn w2(s: Arc<Semaphore>) {
    let permit = s.acquire();
    let mut acc = 0u32;
    acc = acc.wrapping_add(2);
    std::hint::black_box(acc);
    permit.release();
}

fn main() {
    let s = Semaphore::new(1);
    let s1 = Arc::clone(&s);
    let s2 = Arc::clone(&s);
    let t1 = thread::spawn(move || w1(s1));
    let t2 = thread::spawn(move || w2(s2));
    t1.join().unwrap();
    t2.join().unwrap();
    println!("DONE permits=1");
}
