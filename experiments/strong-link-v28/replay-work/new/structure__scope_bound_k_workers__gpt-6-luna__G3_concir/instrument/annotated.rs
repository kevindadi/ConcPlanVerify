mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0#101", 1);

    let w1a = cir_trace::spawn("w1#124", {
        let s = Arc::clone(&s);
        move || w1(s)
    });
    let w2a = cir_trace::spawn("w2#216", {
        let s = Arc::clone(&s);
        move || w2(s)
    });
    let w3a = cir_trace::spawn("w3#308", {
        let s = Arc::clone(&s);
        move || w3(s)
    });

    w1a.join().unwrap();
    w2a.join().unwrap();
    w3a.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}

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
