mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;

fn w1(s: Arc<Semaphore>) {
    let permit = s.acquire();
    // Perform work while holding the permit
    drop(permit);
}

fn w2(s: Arc<Semaphore>) {
    let permit = s.acquire();
    // Perform work while holding the permit
    drop(permit);
}

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 1);

    let s1 = Arc::clone(&s);
    let handle1 = cir_trace::spawn("w1", move || {
        w1(s1);
    });

    let s2 = Arc::clone(&s);
    let handle2 = cir_trace::spawn("w2", move || {
        w2(s2);
    });

    handle1.join().unwrap();
    handle2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
