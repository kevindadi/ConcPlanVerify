mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn w1(s: Arc<Semaphore>) {
    let mut work = 0;
    let permit = s.acquire();
    work += 1;
    permit.release();

    let permit = s.acquire();
    work += 1;
    permit.release();
}

fn w2(s: Arc<Semaphore>) {
    let mut work = 0;
    let permit = s.acquire();
    work += 1;
    permit.release();

    let permit = s.acquire();
    work += 1;
    permit.release();
}

fn main() { crate::cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0#475", 1);

    let worker1 = crate::cir_trace::spawn("w1#502", {
        let s = Arc::clone(&s);
        move || w1(s)
    });
    let worker2 = crate::cir_trace::spawn("w2#598", {
        let s = Arc::clone(&s);
        move || w2(s)
    });

    worker1.join().unwrap();
    worker2.join().unwrap();

    println!("DONE done=1");
 crate::cir_trace::finish();}
