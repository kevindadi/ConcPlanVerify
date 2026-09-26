mod cir_trace;
use concir_sync::Semaphore;
use std::thread;

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 2);

    let w1 = {
        let s = s.clone();
        move || {
            let permit = s.acquire();
            let mut work = 0;
            work = 1;
            permit.release();
            let _ = work;
        }
    };

    let w2 = {
        let s = s.clone();
        move || {
            let permit = s.acquire();
            let mut work = 0;
            work = 1;
            permit.release();
            let _ = work;
        }
    };

    let w3 = {
        let s = s.clone();
        move || {
            let permit = s.acquire();
            let mut work = 0;
            work = 1;
            permit.release();
            let _ = work;
        }
    };

    let h1 = cir_trace::spawn("h1", w1);
    let h2 = cir_trace::spawn("h2", w2);
    let h3 = cir_trace::spawn("h3", w3);

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
