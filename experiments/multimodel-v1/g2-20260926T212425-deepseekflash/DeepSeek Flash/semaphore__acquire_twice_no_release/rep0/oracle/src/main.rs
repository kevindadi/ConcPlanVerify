mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn w1(s: Arc<Semaphore>) {
    let p1 = s.acquire();
    // work while holding permit
    let p2 = s.acquire();
    // more work while holding two permits
    p2.release();
    p1.release();
}

fn w2(s: Arc<Semaphore>) {
    let p1 = s.acquire();
    // work while holding permit
    p1.release();
}

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 2);

    let s1 = Arc::clone(&s);
    let s2 = Arc::clone(&s);

    let h1 = cir_trace::spawn("w1", move || w1(s1));
    let h2 = cir_trace::spawn("w2", move || w2(s2));

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
