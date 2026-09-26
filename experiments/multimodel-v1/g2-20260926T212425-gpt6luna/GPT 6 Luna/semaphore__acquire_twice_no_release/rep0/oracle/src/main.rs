mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn w1(s: Arc<Semaphore>) {
    for _ in 0..2 {
        let permit = s.acquire();
        let _work = (0..1000).fold(0usize, |acc, n| acc.wrapping_add(n));
        permit.release();
    }
}

fn w2(s: Arc<Semaphore>) {
    for _ in 0..2 {
        let permit = s.acquire();
        let _work = (0..1000).fold(0usize, |acc, n| acc.wrapping_add(n));
        permit.release();
    }
}

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 1);

    let s1 = Arc::clone(&s);
    let s2 = Arc::clone(&s);
    let h1 = cir_trace::spawn("w1", move || w1(s1));
    let h2 = cir_trace::spawn("w2", move || w2(s2));

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
