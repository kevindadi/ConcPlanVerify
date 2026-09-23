mod cir_trace;
use concir_sync::Semaphore;
use std::thread;

fn w1(s: &Semaphore) {
    let p = s.acquire();
    p.release();
    let p = s.acquire();
    p.release();
}

fn w2(s: &Semaphore) {
    let p = s.acquire();
    p.release();
    let p = s.acquire();
    p.release();
}

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 1);

    let s_w1 = s.clone();
    let h1 = cir_trace::spawn("w1", move || w1(&s_w1));

    let s_w2 = s.clone();
    let h2 = cir_trace::spawn("w2", move || w2(&s_w2));

    h1.join().unwrap();
    h2.join().unwrap();

    let mut done = 0;
    done = 1;
    println!("DONE done={}", done);
 cir_trace::finish();}
