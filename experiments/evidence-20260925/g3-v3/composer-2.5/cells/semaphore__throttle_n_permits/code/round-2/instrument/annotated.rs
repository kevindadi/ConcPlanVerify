mod cir_trace;
use std::sync::Arc;
use std::thread;

use concir_sync::Semaphore;

fn w1(s: Arc<Semaphore>) {
    s.acquire();
    let mut done = 0;
    done = 1;
    let _ = done;
    s.release();
}

fn w2(s: Arc<Semaphore>) {
    s.acquire();
    let mut done = 0;
    done = 1;
    let _ = done;
    s.release();
}

fn w3(s: Arc<Semaphore>) {
    s.acquire();
    let mut done = 0;
    done = 1;
    let _ = done;
    s.release();
}

fn main() { cir_trace::init();
    let s = Arc::new(Semaphore::new_named("s_semaphore0", 2));

    let s1 = Arc::clone(&s);
    let h1 = cir_trace::spawn("w1", move || w1(s1));

    let s2 = Arc::clone(&s);
    let h2 = cir_trace::spawn("w2", move || w2(s2));

    let s3 = Arc::clone(&s);
    let h3 = cir_trace::spawn("w3", move || w3(s3));

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
