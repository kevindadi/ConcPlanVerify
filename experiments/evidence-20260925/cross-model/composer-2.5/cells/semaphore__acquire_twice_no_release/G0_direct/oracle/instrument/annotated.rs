mod cir_trace;
use concir_sync::Semaphore;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

fn w1(s: Arc<Semaphore>, done: Arc<AtomicUsize>) {
    {
        let _p = s.acquire();
        let _ = done.compare_exchange(0, 1, Ordering::SeqCst, Ordering::SeqCst);
    }
    {
        let _p = s.acquire();
        let _ = done.load(Ordering::SeqCst);
    }
}

fn w2(s: Arc<Semaphore>, done: Arc<AtomicUsize>) {
    {
        let _p = s.acquire();
        let _ = done.compare_exchange(0, 1, Ordering::SeqCst, Ordering::SeqCst);
    }
    {
        let _p = s.acquire();
        let _ = done.load(Ordering::SeqCst);
    }
}

fn main() { cir_trace::init();
    let s = Semaphore::new_named("s_semaphore0", 1);
    let done = Arc::new(AtomicUsize::new(0));

    let s1 = Arc::clone(&s);
    let s2 = Arc::clone(&s);
    let d1 = Arc::clone(&done);
    let d2 = Arc::clone(&done);

    let h1 = cir_trace::spawn("w1", move || w1(s1, d1));
    let h2 = cir_trace::spawn("w2", move || w2(s2, d2));

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done={}", done.load(Ordering::SeqCst));
 cir_trace::finish();}
