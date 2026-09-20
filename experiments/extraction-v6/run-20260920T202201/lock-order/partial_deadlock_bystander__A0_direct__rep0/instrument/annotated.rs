mod cir_trace;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Semaphore {
    permits: Mutex<u32>,
    cv: Condvar,
}

impl Semaphore {
    fn new(count: u32) -> Self {
        Self {
            permits: Mutex::new(count),
            cv: Condvar::new(),
        }
    }

    fn acquire(&self) {
        cir_trace::ev(&cir_trace::tag_str(), "L1"); let mut permits = self.permits.lock().unwrap();
        while *permits == 0 {
            cir_trace::ev(&cir_trace::tag_str(), "L2"); permits = self.cv.wait(permits).unwrap();
        }
        *permits -= 1;
    }

    fn release(&self) {
        cir_trace::ev(&cir_trace::tag_str(), "L3"); *self.permits.lock().unwrap() += 1;
        cir_trace::ev(&cir_trace::tag_str(), "L4"); self.cv.notify_one();
    }
}

fn main() {
    let mtx_a = Arc::new(Mutex::new(()));
    let mtx_b = Arc::new(Mutex::new(()));
    let sem_a = Arc::new(Semaphore::new(0));
    let sem_b = Arc::new(Semaphore::new(0));

    let (ma, mb, sa, sb) = (
        Arc::clone(&mtx_a),
        Arc::clone(&mtx_b),
        Arc::clone(&sem_a),
        Arc::clone(&sem_b),
    );
    cir_trace::ev(&cir_trace::tag_str(), "L5"); let worker_a = thread::spawn(move || {cir_trace::set_tag("tL5"); 
        cir_trace::ev(&cir_trace::tag_str(), "L6"); let ga = ma.lock().unwrap();
        cir_trace::ev(&cir_trace::tag_str(), "L7"); sa.release();
        cir_trace::ev(&cir_trace::tag_str(), "L8"); sb.acquire();
        cir_trace::ev(&cir_trace::tag_str(), "L9"); let gb = mb.lock().unwrap();
        drop(gb);
        drop(ga);
    });

    let (ma2, mb2, sa2, sb2) = (
        Arc::clone(&mtx_a),
        Arc::clone(&mtx_b),
        Arc::clone(&sem_a),
        Arc::clone(&sem_b),
    );
    cir_trace::ev(&cir_trace::tag_str(), "L10"); let worker_b = thread::spawn(move || {cir_trace::set_tag("tL10"); 
        cir_trace::ev(&cir_trace::tag_str(), "L11"); let gb = mb2.lock().unwrap();
        cir_trace::ev(&cir_trace::tag_str(), "L12"); sb2.release();
        cir_trace::ev(&cir_trace::tag_str(), "L13"); sa2.acquire();
        cir_trace::ev(&cir_trace::tag_str(), "L14"); let ga = ma2.lock().unwrap();
        drop(ga);
        drop(gb);
    });

    let done = Arc::new((Mutex::new(false), Condvar::new()));
    let done_clone = Arc::clone(&done);
    cir_trace::ev(&cir_trace::tag_str(), "L15"); let third = thread::spawn(move || {cir_trace::set_tag("tL15"); 
        let (lock, cv) = &*done_clone;
        cir_trace::ev(&cir_trace::tag_str(), "L16"); let mut finished = lock.lock().unwrap();
        while !*finished {
            cir_trace::ev(&cir_trace::tag_str(), "L17"); finished = cv.wait(finished).unwrap();
        }
    });

    cir_trace::ev(&cir_trace::tag_str(), "L18"); worker_a.join().unwrap();
    cir_trace::ev(&cir_trace::tag_str(), "L19"); worker_b.join().unwrap();

    {
        let (lock, cv) = &*done;
        cir_trace::ev(&cir_trace::tag_str(), "L20"); let mut finished = lock.lock().unwrap();
        *finished = true;
        cir_trace::ev(&cir_trace::tag_str(), "L21"); cv.notify_one();
    }
    cir_trace::ev(&cir_trace::tag_str(), "L22"); third.join().unwrap();

    println!("DONE a=1 b=1");
cir_trace::finish(); }
