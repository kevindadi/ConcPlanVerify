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

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0#477", 1);
    let s1 = Arc::clone(&s);
    let s2 = Arc::clone(&s);

    let t1 = cir_trace::spawn("w1#557", move || w1(s1));
    let t2 = cir_trace::spawn("w2#601", move || w2(s2));

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
