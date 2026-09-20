mod cir_trace;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

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
        drop(ga);
        cir_trace::ev(&cir_trace::tag_str(), "L9"); let gb = mb.lock().unwrap();
        drop(gb);
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
        drop(gb);
        cir_trace::ev(&cir_trace::tag_str(), "L14"); let ga = ma2.lock().unwrap();
        drop(ga);
    });

    cir_trace::ev(&cir_trace::tag_str(), "L15"); thread::spawn(move || { cir_trace::set_tag("tL15"); loop {
        thread::sleep(Duration::from_millis(10));
    }});

    cir_trace::ev(&cir_trace::tag_str(), "L16"); worker_a.join().unwrap();
    cir_trace::ev(&cir_trace::tag_str(), "L17"); worker_b.join().unwrap();
    println!("DONE a=1 b=1");
cir_trace::finish(); }
