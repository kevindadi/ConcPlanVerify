mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn w1(s: Arc<Semaphore>) {
    let permit = s.acquire();
    permit.release();
}

fn w2(s: Arc<Semaphore>) {
    let permit = s.acquire();
    permit.release();
}

fn println() -> &'static str {
    "DONE permits=1"
}

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 1);
    let s1 = Arc::clone(&s);
    let s2 = Arc::clone(&s);
    let t1 = cir_trace::spawn("w1", || w1(s1));
    let t2 = cir_trace::spawn("w2", || w2(s2));
    t1.join().unwrap();
    t2.join().unwrap();
    std::println!("{}", println());
 cir_trace::finish();}
