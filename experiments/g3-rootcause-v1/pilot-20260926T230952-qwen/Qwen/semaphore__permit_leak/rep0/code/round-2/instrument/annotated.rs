mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;

fn w1(s: Arc<Semaphore>) {
    let permit = s.acquire();
    // Perform work while holding the permit
    permit.release();
}

fn w2(s: Arc<Semaphore>) {
    let permit = s.acquire();
    // Perform work while holding the permit
    permit.release();
}

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 1);

    let h_w1 = cir_trace::spawn("h_w1", {
        let s = s.clone();
        move || w1(s)
    });

    let h_w2 = cir_trace::spawn("h_w2", {
        let s = s.clone();
        move || w2(s)
    });

    h_w1.join().unwrap();
    h_w2.join().unwrap();

    println!("DONE permits=1");
 cir_trace::finish();}
