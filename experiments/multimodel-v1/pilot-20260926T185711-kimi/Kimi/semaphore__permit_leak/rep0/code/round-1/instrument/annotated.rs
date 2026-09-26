mod cir_trace;
use concir_sync::Semaphore;

fn w1(s: &Semaphore) {
    let permit = s.acquire();
    let mut work = 0;
    work = 1;
    let _ = work;
    permit.release();
}

fn w2(s: &Semaphore) {
    let permit = s.acquire();
    let mut work = 0;
    work = 2;
    let _ = work;
    permit.release();
}

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 1);
    let s1 = s.clone();
    let s2 = s.clone();
    let h1 = cir_trace::spawn("w1", move || w1(&s1));
    let h2 = cir_trace::spawn("w2", move || w2(&s2));
    h1.join().unwrap();
    h2.join().unwrap();
    println!("DONE permits=1");
 cir_trace::finish();}
