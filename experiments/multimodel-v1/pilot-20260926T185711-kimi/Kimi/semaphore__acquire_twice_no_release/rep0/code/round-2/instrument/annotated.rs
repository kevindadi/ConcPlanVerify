mod cir_trace;
use concir_sync::Semaphore;

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 1);

    let w1 = {
        let s = s.clone();
        move || {
            let mut work: i64 = 0;
            let p = s.acquire();
            work += 1;
            p.release();
            let p = s.acquire();
            work += 1;
            p.release();
            let _ = work;
        }
    };

    let w2 = {
        let s = s.clone();
        move || {
            let mut work: i64 = 0;
            let p = s.acquire();
            work += 1;
            p.release();
            let p = s.acquire();
            work += 1;
            p.release();
            let _ = work;
        }
    };

    let h1 = cir_trace::spawn("h1", w1);
    let h2 = cir_trace::spawn("h2", w2);
    h1.join().unwrap();
    h2.join().unwrap();
    println!("DONE done=1");
 cir_trace::finish();}
