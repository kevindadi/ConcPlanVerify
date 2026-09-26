mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;

fn w1(s: Arc<Semaphore>) {
    let mut work: i64 = 0;
    let p = s.acquire();
    work += 1;
    p.release();
    let p = s.acquire();
    work += 1;
    p.release();
    let _ = work;
}

fn w2(s: Arc<Semaphore>) {
    let mut work: i64 = 0;
    let p = s.acquire();
    work += 1;
    p.release();
    let p = s.acquire();
    work += 1;
    p.release();
    let _ = work;
}

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 1);

    let h1 = cir_trace::spawn("h1", {
        let s = s.clone();
        move || w1(s)
    });
    let h2 = cir_trace::spawn("h2", {
        let s = s.clone();
        move || w2(s)
    });

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
