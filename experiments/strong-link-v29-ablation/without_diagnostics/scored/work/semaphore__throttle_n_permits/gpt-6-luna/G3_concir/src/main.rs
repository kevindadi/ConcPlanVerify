mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn w1(s: Arc<Semaphore>) {
    let mut worked = false;
    let permit = s.acquire();
    worked = true;
    permit.release();
}

fn w2(s: Arc<Semaphore>) {
    let mut worked = false;
    let permit = s.acquire();
    worked = true;
    permit.release();
}

fn w3(s: Arc<Semaphore>) {
    let mut worked = false;
    let permit = s.acquire();
    worked = true;
    permit.release();
}

fn main() { crate::cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0#488", 2);

    let t1 = crate::cir_trace::spawn("w1#510", {
        let s = Arc::clone(&s);
        move || w1(s)
    });
    let t2 = crate::cir_trace::spawn("w2#601", {
        let s = Arc::clone(&s);
        move || w2(s)
    });
    let t3 = crate::cir_trace::spawn("w3#692", {
        let s = Arc::clone(&s);
        move || w3(s)
    });

    t1.join().unwrap();
    t2.join().unwrap();
    t3.join().unwrap();

    println!("DONE done=1");
 crate::cir_trace::finish();}
