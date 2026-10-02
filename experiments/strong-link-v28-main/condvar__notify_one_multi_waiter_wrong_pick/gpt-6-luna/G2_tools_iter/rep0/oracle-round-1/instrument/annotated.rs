mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{atomic::{AtomicUsize, Ordering}, Arc};
use std::thread;

struct Semaphore {
    permits: Mutex<usize>,
    changed: Condvar,
}

impl Semaphore {
    fn new(permits: usize) -> Self {
        Self {
            permits: Mutex::new(permits),
            changed: Condvar::new(),
        }
    }

    fn release(&self) {
        let mut permits = self.permits.lock().unwrap();
        *permits += 1;
        self.changed.notify_one();
    }

    fn acquire(&self) {
        let mut permits = self.permits.lock().unwrap();
        while *permits == 0 {
            permits = self.changed.wait(permits).unwrap();
        }
        *permits -= 1;
    }
}

fn waiter(m: Arc<Mutex<bool>>, cv: Arc<Condvar>, g12: Arc<Semaphore>, remaining: Arc<AtomicUsize>) {
    let mut proceed = m.lock().unwrap();

    // Announce readiness while holding m, before waiting on cv.
    g12.release();

    while !*proceed {
        proceed = cv.wait(proceed).unwrap();
    }

    { let __cpv = remaining.fetch_sub(1, Ordering::AcqRel); cir_trace::record_value("remaining#1824", (remaining.load(std::sync::atomic::Ordering::SeqCst)) as i64); __cpv };
}

fn w1(m: Arc<Mutex<bool>>, cv: Arc<Condvar>, g12: Arc<Semaphore>, remaining: Arc<AtomicUsize>) {
    waiter(m, cv, g12, remaining);
}

fn w2(m: Arc<Mutex<bool>>, cv: Arc<Condvar>, g12: Arc<Semaphore>, remaining: Arc<AtomicUsize>) {
    waiter(m, cv, g12, remaining);
}

fn notifier(m: Arc<Mutex<bool>>, cv: Arc<Condvar>, g12: Arc<Semaphore>, gN: Arc<Semaphore>) {
    // Wait until both waiters have announced readiness.
    g12.acquire();
    g12.acquire();

    let mut proceed = m.lock().unwrap();
    *proceed = true;
    cv.notify_all();
    drop(proceed);

    gN.release();
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#1667", false));
    let cv = Arc::new(Condvar::new_named("cv_condvar0#1711"));
    let g12 = Arc::new(Semaphore::new(0));
    let gN = Arc::new(Semaphore::new(0));
    let remaining = Arc::new(AtomicUsize::new(2));

    let t1 = {
        let (m, cv, g12, remaining) =
            (Arc::clone(&m), Arc::clone(&cv), Arc::clone(&g12), Arc::clone(&remaining));
        cir_trace::spawn("w1#2006", move || w1(m, cv, g12, remaining))
    };

    let t2 = {
        let (m, cv, g12, remaining) =
            (Arc::clone(&m), Arc::clone(&cv), Arc::clone(&g12), Arc::clone(&remaining));
        cir_trace::spawn("w2#2213", move || w2(m, cv, g12, remaining))
    };

    let tn = {
        let (m, cv, g12, gN) =
            (Arc::clone(&m), Arc::clone(&cv), Arc::clone(&g12), Arc::clone(&gN));
        cir_trace::spawn("notifier#2406", move || notifier(m, cv, g12, gN))
    };

    gN.acquire();

    t1.join().unwrap();
    t2.join().unwrap();
    tn.join().unwrap();

    println!("DONE waiters={}", remaining.load(Ordering::Acquire));
 cir_trace::finish();}
