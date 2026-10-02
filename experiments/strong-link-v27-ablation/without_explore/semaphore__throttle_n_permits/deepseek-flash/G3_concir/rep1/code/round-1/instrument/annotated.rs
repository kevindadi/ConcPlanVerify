mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn w1(s: Arc<Semaphore>) {
    let permit = s.acquire();
    let mut work = 0;
    work = 1;
    permit.release();
    let _ = work;
}

fn w2(s: Arc<Semaphore>) {
    let permit = s.acquire();
    let mut work = 0;
    work = 1;
    permit.release();
    let _ = work;
}

fn w3(s: Arc<Semaphore>) {
    let permit = s.acquire();
    let mut work = 0;
    work = 1;
    permit.release();
    let _ = work;
}

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0#509", 2);

    let h1 = {
        let s = Arc::clone(&s);
        cir_trace::spawn("w1#573", move || w1(s))
    };
    let h2 = {
        let s = Arc::clone(&s);
        cir_trace::spawn("w2#664", move || w2(s))
    };
    let h3 = {
        let s = Arc::clone(&s);
        cir_trace::spawn("w3#755", move || w3(s))
    };

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
