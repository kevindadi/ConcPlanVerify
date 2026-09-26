mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn do_work(s: Arc<Semaphore>) {
    let permit = s.acquire();

    let mut result = 0usize;
    for i in 0..1_000 {
        result = result.wrapping_add(i);
    }
    std::hint::black_box(result);

    permit.release();
}

fn w1(s: Arc<Semaphore>) {
    do_work(s);
}

fn w2(s: Arc<Semaphore>) {
    do_work(s);
}

fn w3(s: Arc<Semaphore>) {
    do_work(s);
}

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 2);

    let s1 = Arc::clone(&s);
    let s2 = Arc::clone(&s);
    let s3 = Arc::clone(&s);

    let h1 = cir_trace::spawn("w1", move || w1(s1));
    let h2 = cir_trace::spawn("w2", move || w2(s2));
    let h3 = cir_trace::spawn("w3", move || w3(s3));

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
