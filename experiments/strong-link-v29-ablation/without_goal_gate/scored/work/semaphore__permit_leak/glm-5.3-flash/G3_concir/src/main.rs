mod cir_trace;
use concir_sync::Semaphore;
use std::thread;

fn w1(s: &Semaphore) {
    let _permit = s.acquire();
    let mut work: i32 = 0;
    work = work + 1;
    _permit.release();
}

fn w2(s: &Semaphore) {
    let _permit = s.acquire();
    let mut work: i32 = 0;
    work = work + 1;
    _permit.release();
}

fn main() { crate::cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0#337", 1);
    let s1 = s.clone();
    let s2 = s.clone();

    let h1 = crate::cir_trace::spawn("w1#407", move || w1(&s1));
    let h2 = crate::cir_trace::spawn("w2#452", move || w2(&s2));

    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");

    println!("DONE permits=1");
 crate::cir_trace::finish();}
