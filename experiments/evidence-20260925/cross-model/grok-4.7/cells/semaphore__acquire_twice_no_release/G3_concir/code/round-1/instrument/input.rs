use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn main() {
    let s = Semaphore::new(1);
    let s_w1 = Arc::clone(&s);
    let s_w2 = Arc::clone(&s);
    let t1 = thread::spawn(move || w1(s_w1));
    let t2 = thread::spawn(move || w2(s_w2));
    t1.join().unwrap();
    t2.join().unwrap();
    println!("DONE done=1");
}

fn w1(s: Arc<Semaphore>) {
    let permit = s.acquire();
    permit.release();
    let permit = s.acquire();
    permit.release();
}

fn w2(s: Arc<Semaphore>) {
    let permit = s.acquire();
    permit.release();
    let permit = s.acquire();
    permit.release();
}
