mod cir_trace;
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

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 1);
    let s_w1 = Arc::clone(&s);
    let s_w2 = Arc::clone(&s);
    let h1 = cir_trace::spawn("w1", move || w1(s_w1));
    let h2 = cir_trace::spawn("w2", move || w2(s_w2));
    h1.join().unwrap();
    h2.join().unwrap();
    print();
    println!("DONE done=1");
 cir_trace::finish();}
