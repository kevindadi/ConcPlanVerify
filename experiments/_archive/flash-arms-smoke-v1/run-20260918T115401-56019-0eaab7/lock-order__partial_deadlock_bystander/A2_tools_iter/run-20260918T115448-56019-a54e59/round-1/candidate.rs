use std::sync::{Arc, Mutex, Condvar};
use std::thread;

// A simple counting semaphore built from a mutex + condvar.
struct Semaphore {
    count: Mutex<usize>,
    cv: Condvar,
}

impl Semaphore {
    fn new(count: usize) -> Self {
        Semaphore {
            count: Mutex::new(count),
            cv: Condvar::new(),
        }
    }

    fn acquire(&self) {
        let mut c = self.count.lock().unwrap();
        while *c == 0 {
            c = self.cv.wait(c).unwrap();
        }
        *c -= 1;
    }

    fn release(&self) {
        let mut c = self.count.lock().unwrap();
        *c += 1;
        self.cv.notify_one();
    }
}

fn main() {
    // Two mutexes that A and B both need.
    let m1 = Arc::new(Mutex::new(()));
    let m2 = Arc::new(Mutex::new(()));

    // Semaphore handshake: A signals B after acquiring its first lock,
    // B signals A after acquiring its first lock. This orders the
    // acquisition so that no deadlock cycle can form.
    let a_ready = Arc::new(Semaphore::new(0));
    let b_ready = Arc::new(Semaphore::new(0));

    // Bystander keeps making progress forever.
    let bystander = thread::spawn(|| {
        let mut x: u64 = 0;
        loop {
            x = x.wrapping_add(1);
            if x == u64::MAX {
                x = 0;
            }
        }
    });

    let m1_a = Arc::clone(&m1);
    let m2_a = Arc::clone(&m2);
    let a_ready_a = Arc::clone(&a_ready);
    let b_ready_a = Arc::clone(&b_ready);

    let worker_a = thread::spawn(move || {
        // A takes m1 first, then hands off to B, then takes m2.
        let _g1 = m1_a.lock().unwrap();
        a_ready_a.release(); // tell B that A holds m1
        b_ready_a.acquire(); // wait until B holds m2
        let _g2 = m2_a.lock().unwrap();
        // critical section
    });

    let m1_b = Arc::clone(&m1);
    let m2_b = Arc::clone(&m2);
    let a_ready_b = Arc::clone(&a_ready);
    let b_ready_b = Arc::clone(&b_ready);

    let worker_b = thread::spawn(move || {
        // B takes m2 first, then hands off to A, then takes m1.
        let _g2 = m2_b.lock().unwrap();
        b_ready_b.release(); // tell A that B holds m2
        a_ready_b.acquire(); // wait until A holds m1
        let _g1 = m1_b.lock().unwrap();
        // critical section
    });

    worker_a.join().unwrap();
    worker_b.join().unwrap();

    // Bystander runs forever; detach it so main can exit.
    drop(bystander);
}
