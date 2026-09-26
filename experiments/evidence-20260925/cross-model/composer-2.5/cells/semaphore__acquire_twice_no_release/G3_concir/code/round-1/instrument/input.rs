use concir_sync::Semaphore;
use std::sync::Arc;
use std::thread;

fn w1(s: Arc<Semaphore>) {
    let p = s.acquire();
    let () = ();
    p.release();
    let p = s.acquire();
    let () = ();
    p.release();
}

fn w2(s: Arc<Semaphore>) {
    let p = s.acquire();
    let () = ();
    p.release();
}

fn print() {}

fn main() {
    let s = Semaphore::new(1);
    let s_w1 = Arc::clone(&s);
    let s_w2 = Arc::clone(&s);
    let h1 = thread::spawn(move || w1(s_w1));
    let h2 = thread::spawn(move || w2(s_w2));
    h1.join().unwrap();
    h2.join().unwrap();
    print();
    println!("DONE done=1");
}
