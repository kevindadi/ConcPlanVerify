use std::sync::{Arc, Condvar, Mutex};

struct Semaphore {
    count: Mutex<usize>,
    cv: Condvar,
}

impl Semaphore {
    fn new(n: usize) -> Self {
        Semaphore { count: Mutex::new(n), cv: Condvar::new() }
    }

    fn acquire(&self) {
        let mut count = self.count.lock().unwrap();
        while *count == 0 {
            count = self.cv.wait(count).unwrap();
        }
        *count -= 1;
    }

    fn release(&self) {
        let mut count = self.count.lock().unwrap();
        *count += 1;
        self.cv.notify_one();
    }
}

fn worker(name: &'static str, s: Arc<Semaphore>) {
    let rounds: usize = 3;
    for _ in 0..rounds {
        s.acquire(); // hold permit while working (R3)
        // simulate work while holding the permit
        for _ in 0..5 {
            std::hint::spin_loop();
        }
        s.release(); // released on every path, exactly once per acquire (R4)
    }
    let _ = name;
}

fn main() {
    let s = Arc::new(Semaphore::new(1)); // one permit (R2)

    let s1 = Arc::clone(&s);
    let s2 = Arc::clone(&s);

    let h1 = std::thread::spawn(move || worker("w1", s1));
    let h2 = std::thread::spawn(move || worker("w2", s2));

    h1.join().unwrap();
    h2.join().unwrap();

    // R7: exactly `DONE done=1`
    let remaining = *s.count.lock().unwrap();
    println!("DONE done={}", remaining);
}
