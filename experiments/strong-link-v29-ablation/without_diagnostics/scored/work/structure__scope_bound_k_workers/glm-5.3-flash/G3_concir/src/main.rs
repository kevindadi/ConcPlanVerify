mod cir_trace;
use concir_sync::Semaphore;
use std::thread;

fn w1(s: &Semaphore) {
    let _permit = s.acquire();
    // critical section: holding the single permit
    drop(_permit);
}

fn w2(s: &Semaphore) {
    let _permit = s.acquire();
    // critical section: holding the single permit
    drop(_permit);
}

fn w3(s: &Semaphore) {
    let _permit = s.acquire();
    // critical section: holding the single permit
    drop(_permit);
}

fn main() { crate::cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0#462", 1);

    let h1 = {
        let s = s.clone();
        crate::cir_trace::spawn("w1#521", move || w1(&s))
    };
    let h2 = {
        let s = s.clone();
        crate::cir_trace::spawn("w2#608", move || w2(&s))
    };
    let h3 = {
        let s = s.clone();
        crate::cir_trace::spawn("w3#695", move || w3(&s))
    };

    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");
    h3.join().expect("w3 panicked");

    println!("DONE done=1");
 crate::cir_trace::finish();}
