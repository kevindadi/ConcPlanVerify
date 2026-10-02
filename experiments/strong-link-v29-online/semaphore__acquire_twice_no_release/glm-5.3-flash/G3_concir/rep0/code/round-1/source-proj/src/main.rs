use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn w1(s: &Arc<Semaphore>) {
    // acquire #1
    let permit = s.acquire();
    // release #1
    permit.release();
    // acquire #2
    let permit = s.acquire();
    // release #2
    permit.release();
}

fn w2(s: &Arc<Semaphore>) {
    // acquire #1
    let permit = s.acquire();
    // release #1
    permit.release();
    // acquire #2
    let permit = s.acquire();
    // release #2
    permit.release();
}

fn main() {
    // Shared counting permit pool with exactly one initial permit.
    let s: Arc<Semaphore> = Semaphore::new(1);

    let s1 = Arc::clone(&s);
    let s2 = Arc::clone(&s);

    let h1 = thread::spawn(move || w1(&s1));
    let h2 = thread::spawn(move || w2(&s2));

    // Supervising task waits for both workers to finish.
    h1.join().expect("w1 panicked");
    h2.join().expect("w2 panicked");

    println!("DONE done=1");
}
