use concir_sync::Semaphore;
use std::thread;

fn w1(s_kept: std::sync::Arc<Semaphore>) {
    let permit = s_kept.acquire();
    permit.release();
}

fn w2(s_kept: std::sync::Arc<Semaphore>) {
    let permit = s_kept.acquire();
    permit.release();
}

fn main() {
    let s_kept = Semaphore::new(1);

    let s1 = s_kept.clone();
    let h1 = thread::spawn(move || w1(s1));

    let s2 = s_kept.clone();
    let h2 = thread::spawn(move || w2(s2));

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
}
