use concir_sync::Semaphore;
use std::thread;

fn w1(s: Semaphore) {
    s.acquire();
    let mut work = 0;
    work = 1;
    s.release();

    s.acquire();
    work = 2;
    s.release();
}

fn w2(s: Semaphore) {
    s.acquire();
    let mut work = 0;
    work = 1;
    s.release();

    s.acquire();
    work = 2;
    s.release();
}

fn main() {
    let s = Semaphore::new(1);

    let s_w1 = s.clone();
    let h1 = thread::spawn(move || w1(s_w1));

    let s_w2 = s.clone();
    let h2 = thread::spawn(move || w2(s_w2));

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
}
