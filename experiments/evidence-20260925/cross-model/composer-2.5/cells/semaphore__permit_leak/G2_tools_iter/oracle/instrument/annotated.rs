mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn w1(s: Arc<Semaphore>) {
    let permit = s.acquire();
    // work while holding the permit
    permit.release();
}

fn w2(s: Arc<Semaphore>) {
    let permit = s.acquire();
    // work while holding the permit
    permit.release();
}

fn main() { cir_trace::init();
    let supervisor = cir_trace::spawn("new", || {
        let s = Semaphore::new_named("s_semaphore0", 1);
        let s1 = Arc::clone(&s);
        let s2 = Arc::clone(&s);

        let h1 = cir_trace::spawn("w1", move || w1(s1));
        let h2 = cir_trace::spawn("w2", move || w2(s2));

        h1.join().unwrap();
        h2.join().unwrap();

        println!("DONE permits=1");
    });

    supervisor.join().unwrap();
 cir_trace::finish();}
