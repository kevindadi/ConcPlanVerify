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

fn w3(s: Arc<Semaphore>) {
    let permit = s.acquire();
    permit.release();
}

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0#347", 1);

    let s1 = Arc::clone(&s);
    let s2 = Arc::clone(&s);
    let s3 = Arc::clone(&s);

    let t1 = cir_trace::spawn("w1#457", move || w1(s1));
    let t2 = cir_trace::spawn("w2#501", move || w2(s2));
    let t3 = cir_trace::spawn("w3#545", move || w3(s3));

    t1.join().unwrap();
    t2.join().unwrap();
    t3.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
