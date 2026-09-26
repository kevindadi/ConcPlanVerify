mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 1);
    let s_w1 = Arc::clone(&s);
    let s_w2 = Arc::clone(&s);
    let t1 = cir_trace::spawn("w1", move || w1(s_w1));
    let t2 = cir_trace::spawn("w2", move || w2(s_w2));
    t1.join().unwrap();
    t2.join().unwrap();
    println!("DONE done=1");
 cir_trace::finish();}

fn w1(s: Arc<Semaphore>) {
    let permit = s.acquire();
    permit.release();
    let permit = s.acquire();
    permit.release();
}

fn w2(s: Arc<Semaphore>) {
    let permit = s.acquire();
    permit.release();
    let permit = s.acquire();
    permit.release();
}
