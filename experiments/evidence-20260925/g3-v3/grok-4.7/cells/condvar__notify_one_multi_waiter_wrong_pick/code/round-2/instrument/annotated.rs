mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc};
use std::thread;

struct Semaphore {
    count: AtomicI32,
}

impl Semaphore {
    fn new(count: i32) -> Self {
        Semaphore {
            count: AtomicI32::new(count),
        }
    }

    fn acquire(&self) {
        loop {
            match self.count.fetch_update(Ordering::AcqRel, Ordering::Acquire, |c| {
                if c > 0 { Some(c - 1) } else { None }
            }) {
                Ok(_) => return,
                Err(_) => thread::yield_now(),
            }
        }
    }

    fn release(&self) {
        self.count.fetch_add(1, Ordering::Release);
    }
}

struct Shared {
    m: Mutex<()>,
    cv: Condvar,
    g12: Semaphore,
    gN: Semaphore,
}

fn w1(m: &Mutex<()>, cv: &Condvar, g12: &Semaphore, gN: &Semaphore) {
    let guard = m.lock().unwrap();
    g12.release();
    let guard = cv.wait(guard).unwrap();
    drop(guard);
    gN.release();
}

fn w2(m: &Mutex<()>, cv: &Condvar, g12: &Semaphore, gN: &Semaphore) {
    let guard = m.lock().unwrap();
    g12.release();
    let guard = cv.wait(guard).unwrap();
    drop(guard);
    gN.release();
}

fn notifier(m: &Mutex<()>, cv: &Condvar, g12: &Semaphore, gN: &Semaphore) {
    g12.acquire();
    g12.acquire();
    let guard = m.lock().unwrap();
    cv.notify_one();
    cv.notify_one();
    drop(guard);
    gN.acquire();
    gN.acquire();
}

fn main() { cir_trace::init();
    let shared = Arc::new(Shared {
        m: Mutex::new_named("shared_mutex0", ()),
        cv: Condvar::new_named("shared_condvar0"),
        g12: Semaphore::new(0),
        gN: Semaphore::new(0),
    });

    let s1 = Arc::clone(&shared);
    let h1 = cir_trace::spawn("w1", move || w1(&s1.m, &s1.cv, &s1.g12, &s1.gN));

    let s2 = Arc::clone(&shared);
    let h2 = cir_trace::spawn("w2", move || w2(&s2.m, &s2.cv, &s2.g12, &s2.gN));

    let sn = Arc::clone(&shared);
    let hn = cir_trace::spawn("notifier", move || notifier(&sn.m, &sn.cv, &sn.g12, &sn.gN));

    h1.join().unwrap();
    h2.join().unwrap();
    hn.join().unwrap();

    println!("DONE waiters=0");
 cir_trace::finish();}
