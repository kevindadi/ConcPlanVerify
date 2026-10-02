mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn w1(s: Arc<Semaphore>) {
    let mut tmp: i32 = 0;

    {
        let permit = s.acquire();
        tmp = 1;
        permit.release();
    }

    {
        let permit = s.acquire();
        tmp = 2;
        permit.release();
    }
}

fn w2(s: Arc<Semaphore>) {
    let mut tmp: i32 = 0;

    {
        let permit = s.acquire();
        tmp = 3;
        permit.release();
    }

    {
        let permit = s.acquire();
        tmp = 4;
        permit.release();
    }
}

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0#573", 1);

    let s1 = Arc::clone(&s);
    let t1 = cir_trace::spawn("w1#624", move || w1(s1));

    let s2 = Arc::clone(&s);
    let t2 = cir_trace::spawn("w2#698", move || w2(s2));

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
