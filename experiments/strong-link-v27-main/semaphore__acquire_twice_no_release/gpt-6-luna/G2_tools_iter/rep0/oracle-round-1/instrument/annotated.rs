mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};

struct Semaphore {
    permits: Mutex<usize>,
    available: Condvar,
}

impl Semaphore {
    fn new(permits: usize) -> Self {
        Self {
            permits: Mutex::new(permits),
            available: Condvar::new(),
        }
    }

    fn acquire(&self) -> Permit<'_> {
        let mut permits = self.permits.lock().unwrap();

        while *permits == 0 {
            permits = self.available.wait(permits).unwrap();
        }

        *permits -= 1;
        Permit { semaphore: self }
    }

    fn release(&self) {
        let mut permits = self.permits.lock().unwrap();
        *permits += 1;
        self.available.notify_one();
    }
}

struct Permit<'a> {
    semaphore: &'a Semaphore,
}

impl Drop for Permit<'_> {
    fn drop(&mut self) {
        self.semaphore.release();
    }
}

fn do_work() {
    let mut value = 0usize;
    for i in 0..10_000 {
        value = value.wrapping_add(i);
    }
    std::hint::black_box(value);
}

fn w1(s: Arc<Semaphore>) {
    for _ in 0..2 {
        let _permit = s.acquire();
        do_work();
    }
}

fn w2(s: Arc<Semaphore>) {
    for _ in 0..2 {
        let _permit = s.acquire();
        do_work();
    }
}

fn main() { cir_trace::init();
    let s = Arc::new(Semaphore::new(1));

    let s1 = Arc::clone(&s);
    let s2 = Arc::clone(&s);

    let worker1 = std::thread::Builder::new()
        .name("w1".to_string())
        .spawn(move || w1(s1))
        .unwrap();

    let worker2 = std::thread::Builder::new()
        .name("w2".to_string())
        .spawn(move || w2(s2))
        .unwrap();

    worker1.join().unwrap();
    worker2.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
