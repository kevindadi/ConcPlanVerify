use std::sync::Arc;
use std::thread;

use concir_sync::Semaphore;

fn w1(s: &Arc<Semaphore>) {
    let _permit = s.acquire();
    // critical section: worker performs its work while holding one permit
    drop(_permit);
}

fn w2(s: &Arc<Semaphore>) {
    let _permit = s.acquire();
    // critical section: worker performs its work while holding one permit
    drop(_permit);
}

fn w3(s: &Arc<Semaphore>) {
    let _permit = s.acquire();
    // critical section: worker performs its work while holding one permit
    drop(_permit);
}

fn main() {
    let s = Semaphore::new(2);
    let h1 = thread::spawn(move || w1(&s));
    let h2 = thread::spawn(move || w2(&s));
    let h3 = thread::spawn(move || w3(&s));
    h1.join().unwrap();
    h2.join().unwrap();
    h3.join().unwrap();
    println!("DONE done=1");
}
