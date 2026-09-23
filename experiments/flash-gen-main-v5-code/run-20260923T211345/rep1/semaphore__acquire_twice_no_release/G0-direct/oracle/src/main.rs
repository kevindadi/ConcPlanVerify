mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn worker(name: &'static str, s: Arc<Semaphore>) {
    // Acquire the permit, do work, release it.
    {
        let _permit = s.acquire();
        // critical section: only one worker at a time
        println!("{} working", name);
    }

    // Acquire a second time to demonstrate multiple acquisitions.
    {
        let _permit = s.acquire();
        println!("{} working again", name);
    }
}

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 1);

    let s1 = Arc::clone(&s);
    let s2 = Arc::clone(&s);

    let w1 = cir_trace::spawn("worker", move || worker("w1", s1));
    let w2 = cir_trace::spawn("worker", move || worker("w2", s2));

    w1.join().unwrap();
    w2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
