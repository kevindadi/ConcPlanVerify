mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Semaphore {
    count: Mutex<usize>,
    cv: Condvar,
}

impl Semaphore {
    fn new(count: usize) -> Self {
        Semaphore { count: Mutex::new(count), cv: Condvar::new() }
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

fn main() { cir_trace::init();
    let a = Arc::new(Mutex::new_named("a_mutex0", ()));
    let b = Arc::new(Mutex::new_named("b_mutex0", ()));
    let sa = Arc::new(Semaphore::new(0));
    let sb = Arc::new(Semaphore::new(0));
    let done = Arc::new(Semaphore::new(0));
    let flag = Arc::new(Mutex::new_named("flag_mutex0", 0u32));

    // Worker a: takes a first, then b.
    let a_lock = Arc::clone(&a);
    let b_lock = Arc::clone(&b);
    let sa_sem = Arc::clone(&sa);
    let sb_sem = Arc::clone(&sb);
    let done_sem = Arc::clone(&done);
    let flag_a = Arc::clone(&flag);

    let worker_a = cir_trace::spawn("worker_a", move || {
        let ga = a_lock.lock().unwrap();
        { let mut f = flag_a.lock().unwrap(); *f += 1; }
        sa_sem.release();          // a took its first lock
        sb_sem.acquire();          // wait until b took its first lock
        let gb = b_lock.lock().unwrap();
        // critical section: both locks held
        drop(gb);
        drop(ga);
        done_sem.release();        // a finished, b may proceed
    });

    // Worker b: takes b first, then a.
    let a_lock_b = Arc::clone(&a);
    let b_lock_b = Arc::clone(&b);
    let sa_sem_b = Arc::clone(&sa);
    let sb_sem_b = Arc::clone(&sb);
    let done_sem_b = Arc::clone(&done);
    let flag_b = Arc::clone(&flag);

    let worker_b = cir_trace::spawn("worker_b", move || {
        let gb = b_lock_b.lock().unwrap();
        { let mut f = flag_b.lock().unwrap(); *f += 1; }
        sb_sem_b.release();        // b took its first lock
        sa_sem_b.acquire();        // wait until a took its first lock
        drop(gb);                  // release b so a can take it
        done_sem_b.acquire();      // wait until a finished
        let ga = a_lock_b.lock().unwrap();
        let gb2 = b_lock_b.lock().unwrap();
        // critical section: both locks held
        drop(gb2);
        drop(ga);
    });

    // Bystander: keeps making progress, never finishes on its own.
    let flag_by = Arc::clone(&flag);
    let stop = Arc::new(Mutex::new_named("stop_mutex0", false));
    let stop_by = Arc::clone(&stop);
    let bystander = cir_trace::spawn("yield_now", move || loop {
        {
            let s = stop_by.lock().unwrap();
            if *s {
                break;
            }
        }
        let mut f = flag_by.lock().unwrap();
        *f = f.wrapping_add(1);
        drop(f);
        thread::yield_now();
    });

    worker_a.join().unwrap();
    worker_b.join().unwrap();

    // Signal the bystander to stop, then join it so no thread is leaked.
    {
        let mut s = stop.lock().unwrap();
        *s = true;
    }
    bystander.join().unwrap();

    println!("DONE a=1 b=1");
 cir_trace::finish();}
