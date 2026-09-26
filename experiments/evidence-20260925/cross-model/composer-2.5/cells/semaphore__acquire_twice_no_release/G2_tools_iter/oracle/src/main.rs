mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn w1(s: Arc<Semaphore>) {
    {
        let _p = s.acquire();
        // critical section
    }
    {
        let _p = s.acquire();
        // critical section
    }
}

fn w2(s: Arc<Semaphore>) {
    {
        let _p = s.acquire();
        // critical section
    }
    {
        let _p = s.acquire();
        // critical section
    }
}

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 1);

    let s1 = Arc::clone(&s);
    let j1 = cir_trace::spawn("w1", move || w1(s1));

    let s2 = Arc::clone(&s);
    let j2 = cir_trace::spawn("w2", move || w2(s2));

    j1.join().unwrap();
    j2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
