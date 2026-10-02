mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn w1(s: Arc<Semaphore>) {
    let permit = s.acquire();
    let mut work = 0;
    work = work + 1;
    permit.release();
}

fn w2(s: Arc<Semaphore>) {
    let permit = s.acquire();
    let mut work = 0;
    work = work + 1;
    permit.release();
}

fn main() { crate::cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0#351", 1);

    let t1 = crate::cir_trace::spawn("w1#373", {
        let s = Arc::clone(&s);
        move || w1(s)
    });
    let t2 = crate::cir_trace::spawn("w2#464", {
        let s = Arc::clone(&s);
        move || w2(s)
    });

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE permits=1");
 crate::cir_trace::finish();}
