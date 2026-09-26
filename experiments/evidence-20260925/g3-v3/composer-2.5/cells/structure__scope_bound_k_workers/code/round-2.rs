use concir_sync::Semaphore;
use std::thread;

fn w1(s: &Semaphore) {
    s.acquire();
    s.release();
}

fn w2(s: &Semaphore) {
    s.acquire();
    s.release();
}

fn w3(s: &Semaphore) {
    s.acquire();
    s.release();
}

fn main() {
    let s = Semaphore::new(1);

    thread::scope(|scope| {
        scope.spawn(|| w1(&s));
        scope.spawn(|| w2(&s));
        scope.spawn(|| w3(&s));
    });

    println!("DONE done=1");
}
