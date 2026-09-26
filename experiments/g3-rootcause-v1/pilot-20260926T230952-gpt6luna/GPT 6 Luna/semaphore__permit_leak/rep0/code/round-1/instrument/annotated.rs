mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn w1(s: Arc<Semaphore>) {
    let mut work = 0;
    let permit = s.acquire();
    work = 1;
    permit.release();
}

fn w2(s: Arc<Semaphore>) {
    let mut work = 0;
    let permit = s.acquire();
    work = 1;
    permit.release();
}

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 1);

    let s1 = Arc::clone(&s);
    let thread1 = cir_trace::spawn("w1", move || w1(s1));

    let s2 = Arc::clone(&s);
    let thread2 = cir_trace::spawn("w2", move || w2(s2));

    thread1.join().unwrap();
    thread2.join().unwrap();

    println!("DONE permits=1");
 cir_trace::finish();}
