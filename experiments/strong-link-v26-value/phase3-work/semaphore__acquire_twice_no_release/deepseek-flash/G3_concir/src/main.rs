mod cir_trace;
use std::sync::Arc;
use std::thread;
use concir_sync::Semaphore;

fn w1(s: Arc<Semaphore>) {
    let p1 = s.acquire();
    p1.release();

    let p2 = s.acquire();
    p2.release();
}

fn w2(s: Arc<Semaphore>) {
    let p1 = s.acquire();
    p1.release();

    let p2 = s.acquire();
    p2.release();
}

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0#339", 1);

    let s1 = Arc::clone(&s);
    let h1 = cir_trace::spawn("w1#390", move || w1(s1));

    let s2 = Arc::clone(&s);
    let h2 = cir_trace::spawn("w2#464", move || w2(s2));

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
