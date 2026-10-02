mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0#101", 1);

    let s1 = Arc::clone(&s);
    let h1 = cir_trace::spawn("w1#152", move || w1(s1));

    let s2 = Arc::clone(&s);
    let h2 = cir_trace::spawn("w2#226", move || w2(s2));

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}

fn w1(s: Arc<Semaphore>) {
    let mut work: i32 = 0;

    let permit = s.acquire();
    work = 1;
    permit.release();

    let permit = s.acquire();
    work = 2;
    permit.release();
}

fn w2(s: Arc<Semaphore>) {
    let mut work: i32 = 0;

    let permit = s.acquire();
    work = 10;
    permit.release();

    let permit = s.acquire();
    work = 20;
    permit.release();
}
