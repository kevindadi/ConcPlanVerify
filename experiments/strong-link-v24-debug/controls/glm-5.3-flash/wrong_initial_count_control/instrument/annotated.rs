mod cir_trace;
use concir_sync::Semaphore;

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0#64", 2);

    let s1 = s.clone();
    let h1 = cir_trace::spawn("w1#110", move || {
        w1(&s1);
    });

    let s2 = s.clone();
    let h2 = cir_trace::spawn("w2#202", move || {
        w2(&s2);
    });

    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");

    println!("DONE done=1");
 cir_trace::finish();}

fn w1(s: &Semaphore) {
    let mut work: i32 = 0;

    let permit = s.acquire();
    work = work + 1;
    permit.release();

    let permit = s.acquire();
    work = work + 1;
    permit.release();
}

fn w2(s: &Semaphore) {
    let mut work: i32 = 0;

    let permit = s.acquire();
    work = work + 1;
    permit.release();

    let permit = s.acquire();
    work = work + 1;
    permit.release();
}
