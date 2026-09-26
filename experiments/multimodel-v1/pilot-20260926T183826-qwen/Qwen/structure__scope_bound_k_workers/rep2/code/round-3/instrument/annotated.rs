mod cir_trace;
use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn w1(s: Arc<Semaphore>) {
    let permit = s.acquire();
    // Work is done while holding the permit.
    drop(permit); // Releases the semaphore early, equivalent to semaphore_release
}

fn w2(s: Arc<Semaphore>) {
    let permit = s.acquire();
    // Work is done while holding the permit.
    drop(permit); // Releases the semaphore early, equivalent to semaphore_release
}

fn w3(s: Arc<Semaphore>) {
    let permit = s.acquire();
    // Work is done while holding the permit.
    drop(permit); // Releases the semaphore early, equivalent to semaphore_release
}

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 1);

    let h1 = cir_trace::spawn("h1", {
        let s = Arc::clone(&s);
        move || w1(s)
    });

    let h2 = cir_trace::spawn("h2", {
        let s = Arc::clone(&s);
        move || w2(s)
    });

    let h3 = cir_trace::spawn("h3", {
        let s = Arc::clone(&s);
        move || w3(s)
    });

    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
