mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn w1(s: Arc<Semaphore>) {
    let mut tmp = 0;

    let permit = s.acquire();
    tmp = 1;
    permit.release();

    let permit = s.acquire();
    tmp = 2;
    permit.release();
}

fn w2(s: Arc<Semaphore>) {
    let mut tmp = 0;

    let permit = s.acquire();
    tmp = 3;
    permit.release();

    let permit = s.acquire();
    tmp = 4;
    permit.release();
}

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0#467", 1);

    let s1 = Arc::clone(&s);
    let s2 = Arc::clone(&s);

    let h1 = cir_trace::spawn("w1#548", move || w1(s1));
    let h2 = cir_trace::spawn("w2#592", move || w2(s2));

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
