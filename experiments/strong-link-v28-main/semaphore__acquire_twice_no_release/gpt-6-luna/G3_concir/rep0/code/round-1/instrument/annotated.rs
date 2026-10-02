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

    let _ = work;
}

fn w2(s: Arc<Semaphore>) {
    let mut work = 0;

    let permit = s.acquire();
    work += 1;
    permit.release();

    let permit = s.acquire();
    work += 1;
    permit.release();

    let _ = work;
}

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0#515", 1);

    let worker1 = cir_trace::spawn("w1#542", {
        let s = Arc::clone(&s);
        move || w1(s)
    });
    let worker2 = cir_trace::spawn("w2#638", {
        let s = Arc::clone(&s);
        move || w2(s)
    });

    worker1.join().unwrap();
    worker2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
