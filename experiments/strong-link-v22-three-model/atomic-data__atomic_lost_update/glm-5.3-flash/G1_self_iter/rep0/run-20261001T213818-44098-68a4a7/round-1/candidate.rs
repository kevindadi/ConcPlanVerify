use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

fn worker(id: &'static str, c: &Arc<AtomicUsize>) {
    // R4/R5: retry loop; CAS ensures indivisible read-modify-write (R7),
    // so no lost updates and every increment eventually lands (R6).
    loop {
        let cur = c.load(Ordering::Relaxed);
        match c.compare_exchange_weak(
            cur,
            cur + 1,
            Ordering::AcqRel,
            Ordering::Relaxed,
        ) {
            Ok(_) => break,
            Err(_) => continue, // failed attempt retried, never abandoned
        }
    }
    let _ = id;
}

fn main() {
    // R2: single shared counter starting at zero
    let c = Arc::new(AtomicUsize::new(0));

    // R1: supervising task launches both workers and j/ts them
    let h = [
        {
            let c = Arc::clone(&c);
            std::thread::spawn(|| { w1(&c); })
        },
        {
            let c = Arc::clone(&c);
            std::thread::spawn(|| { w2(&c); })
        },
    ];

    fn w1(c: &Arc<AtomicUsize>) { worker("w1", c); }
    fn w2(c: &Arc<AtomicUsize>) { worker("w2", c); }

    for handle in &h {
        handle.join().expect("worker panicked");
    }

    // R3: both done, c == 2.
    debug_assert_eq!(c.load(Ordering::SeqCst), 2);
    // R8: no loops can spin forever; CAS fails only when another thread
    // succeeded, reducing failure opportunities to a finite number.
    // R9: exact required output line.
    println!("DONE done=1");
}
