mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn w1(s_kept: Arc<Semaphore>) {
    let permit = s_kept.acquire();
    permit.release();
}

fn w2(s_kept: Arc<Semaphore>) {
    let permit = s_kept.acquire();
    permit.release();
}

fn main() { cir_trace::init();
    let s_kept = Semaphore::new_named("s_kept_semaphore0", 1);

    let s1 = Arc::clone(&s_kept);
    let s2 = Arc::clone(&s_kept);

    let h1 = cir_trace::spawn("w1", move || w1(s1));
    let h2 = cir_trace::spawn("w2", move || w2(s2));

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
