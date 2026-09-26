use std::sync::{Arc, Semaphore};
use std::thread;

fn w1(s: Arc<Semaphore>) {
    let _permit = s.acquire().unwrap();
}

fn w2(s: Arc<Semaphore>) {
    let _permit = s.acquire().unwrap();
}

fn w3(s: Arc<Semaphore>) {
    let _permit = s.acquire().unwrap();
}

fn main() {
    let s = Arc::new(Semaphore::new(1));

    thread::scope(|scope| {
        scope.spawn({
            let s = Arc::clone(&s);
            move || w1(s)
        });
        scope.spawn({
            let s = Arc::clone(&s);
            move || w2(s)
        });
        scope.spawn({
            let s = Arc::clone(&s);
            move || w3(s)
        });
    });

    println!("DONE done=1");
}
