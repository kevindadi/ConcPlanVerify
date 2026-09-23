mod cir_trace;
use concir_sync::Semaphore;
use std::thread;

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 1);

    let s1 = s.clone();
    let w1 = cir_trace::spawn("w1", move || {
        let _permit = s1.acquire();
        // work while holding the permit
        // permit released on drop before finishing
    });

    let s2 = s.clone();
    let w2 = cir_trace::spawn("w2", move || {
        let _permit = s2.acquire();
        // work while holding the permit
        // permit released on drop before finishing
    });

    w1.join().unwrap();
    w2.join().unwrap();

    println!("DONE permits=1");
 cir_trace::finish();}
