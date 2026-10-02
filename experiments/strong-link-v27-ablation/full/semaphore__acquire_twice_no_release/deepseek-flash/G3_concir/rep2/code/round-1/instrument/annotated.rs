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
        tmp = 1;
        permit.release();
    }

    {
        let permit = s.acquire();
        tmp = 2;
        permit.release();
    }
}

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0#573", 1);

    let s_w1 = Arc::clone(&s);
    let s_w2 = Arc::clone(&s);

    let h1 = cir_trace::spawn("w1#658", move || w1(s_w1));
    let h2 = cir_trace::spawn("w2#704", move || w2(s_w2));

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
