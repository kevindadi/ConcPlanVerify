mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn w1(s: Arc<Semaphore>) {
    let _permit = s.acquire();
    s.release();
}

fn w2(s: Arc<Semaphore>) {
    let _permit = s.acquire();
    s.release();
}

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 1);

    let s1 = Arc::clone(&s);
    let s2 = Arc::clone(&s);

    let t1 = cir_trace::spawn("w1", move || w1(s1));
    let t2 = cir_trace::spawn("w2", move || w2(s2));

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE permits=1");
 cir_trace::finish();}
