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
    let s = Semaphore::new_named("s_semaphore0", 2);

    let w1 = cir_trace::spawn("w1", {
        let s = s.clone();
        move || w1(s)
    });
    let w2 = cir_trace::spawn("w2", {
        let s = s.clone();
        move || w2(s)
    });
    let w3 = cir_trace::spawn("w3", {
        let s = s.clone();
        move || w3(s)
    });

    w1.join().unwrap();
    w2.join().unwrap();
    w3.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
