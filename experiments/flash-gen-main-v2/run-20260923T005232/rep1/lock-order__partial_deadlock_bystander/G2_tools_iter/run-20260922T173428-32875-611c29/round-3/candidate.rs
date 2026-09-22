use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Semaphore {
    count: Mutex<usize>,
    cv: Condvar,
}

impl Semaphore {
    fn new(n: usize) -> Self {
        Semaphore {
            count: Mutex::new(n),
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
    let a = Arc::new(Mutex::new(()));
    let b = Arc::new(Mutex::new(()));

    let sa = Arc::new(Semaphore::new(0));
    let sb = Arc::new(Semaphore::new(0));

    let flag = Arc::new(Mutex::new(0u32));
    let stop = Arc::new(AtomicBool::new(false));

    let mut handles = Vec::new();

    // Worker a: handshake first (signal sa, wait for sb), then acquire the
    // two locks in the global order a -> b. Holding no lock while waiting
    // for the permit means worker b can always make progress.
    {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        let sa = Arc::clone(&sa);
        let sb = Arc::clone(&sb);
        let flag = Arc::clone(&flag);

        handles.push(thread::spawn(move || {
            // Handshake: announce that we are taking our first lock, then
            // wait until the other worker has taken its first lock.
            sa.release();
            sb.acquire();

            // Critical section: both locks held simultaneously, acquired in
            // the consistent global order a -> b.
            let ga = a.lock().unwrap();
            let gb = b.lock().unwrap();

            {
                let mut f = flag.lock().unwrap();
                *f |= 1;
            }

            drop(gb);
            drop(ga);
        }));
    }

    // Worker b: handshake first (signal sb, wait for sa), then acquire the
    // two locks in the same global order a -> b. No lock-order inversion.
    {
        let a = Arc::clone(&a);
        let b = Arc::clone(&b);
        let sa = Arc::clone(&sa);
        let sb = Arc::clone(&sb);
        let flag = Arc::clone(&flag);

        handles.push(thread::spawn(move || {
            sb.release();
            sa.acquire();

            let ga = a.lock().unwrap();
            let gb = b.lock().unwrap();

            {
                let mut f = flag.lock().unwrap();
                *f |= 2;
            }

            drop(gb);
            drop(ga);
        }));
    }

    // Bystander task: keeps making progress without ever finishing on its
    // own. It only touches `flag` briefly and never holds a lock while
    // waiting, so it cannot block the workers. Main signals it to stop once
    // the workers are done.
    let bystander = {
        let flag = Arc::clone(&flag);
        let stop = Arc::clone(&stop);
        thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                let mut f = flag.lock().unwrap();
                *f ^= 0;
                drop(f);
                thread::yield_now();
            }
        })
    };

    for h in handles {
        h.join().unwrap();
    }

    // Workers are finished; tell the bystander to stop and reap it so no
    // thread is left running when the process exits.
    stop.store(true, Ordering::Relaxed);
    bystander.join().unwrap();

    let f = flag.lock().unwrap();
    let a_val = *f & 1;
    let b_val = (*f >> 1) & 1;
    println!("DONE a={} b={}", a_val, b_val);
}
