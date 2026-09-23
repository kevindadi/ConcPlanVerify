mod cir_trace;
use concir_sync::Semaphore;
use std::thread;

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 1);

    let s1 = s.clone();
    let w1 = cir_trace::spawn("w1", move || {
        let p1 = s1.acquire();
        // work while holding permit
        drop(p1);

        let p2 = s1.acquire();
        // work while holding permit
        drop(p2);
    });

    let s2 = s.clone();
    let w2 = cir_trace::spawn("w2", move || {
        let p1 = s2.acquire();
        // work while holding permit
        drop(p1);

        let p2 = s2.acquire();
        // work while holding permit
        drop(p2);
    });

    w1.join().unwrap();
    w2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
