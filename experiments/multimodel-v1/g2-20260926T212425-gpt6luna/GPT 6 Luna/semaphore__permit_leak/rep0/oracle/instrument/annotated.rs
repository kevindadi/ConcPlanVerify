mod cir_trace;
use concir_sync::Semaphore;
use std::thread;

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 1);

    let s_w1 = s.clone();
    let w1 = cir_trace::spawn("black_box", move || {
        let permit = s_w1.acquire();
        let _work = std::hint::black_box((0..1_000_u64).sum::<u64>());
        permit.release();
    });

    let s_w2 = s.clone();
    let w2 = cir_trace::spawn("black_box", move || {
        let permit = s_w2.acquire();
        let _work = std::hint::black_box((0..1_000_u64).sum::<u64>());
        permit.release();
    });

    w1.join().unwrap();
    w2.join().unwrap();

    println!("DONE permits=1");
 cir_trace::finish();}
