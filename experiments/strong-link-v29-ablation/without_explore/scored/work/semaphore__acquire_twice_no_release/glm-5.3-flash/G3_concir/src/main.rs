mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn w1(s: &Arc<Semaphore>) {
    let mut i: i32 = 0;
    loop {
        let permit = s.acquire();
        i = i + 1;
        permit.release();
        if !(i < 2) {
            break;
        }
    }
}

fn w2(s: &Arc<Semaphore>) {
    let mut i: i32 = 0;
    loop {
        let permit = s.acquire();
        i = i + 1;
        permit.release();
        if !(i < 2) {
            break;
        }
    }
}

fn main() { crate::cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0#505", 1);
    let s1 = Arc::clone(&s);
    let s2 = Arc::clone(&s);
    let h1 = crate::cir_trace::spawn("w1#584", move || w1(&s1));
    let h2 = crate::cir_trace::spawn("w2#629", move || w2(&s2));
    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");
    println!("DONE done=1");
 crate::cir_trace::finish();}
