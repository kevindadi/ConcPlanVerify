mod cir_trace;
mod concir_sync;
mod concir_sync;

use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn w1(s: Arc<Semaphore>) {
    let _permit = s.acquire();
    drop(_permit);
}

fn w2(s: Arc<Semaphore>) {
    let _permit = s.acquire();
    drop(_permit);
}

fn w3(s: Arc<Semaphore>) {
    let _permit = s.acquire();
    drop(_permit);
}

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 1);

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
