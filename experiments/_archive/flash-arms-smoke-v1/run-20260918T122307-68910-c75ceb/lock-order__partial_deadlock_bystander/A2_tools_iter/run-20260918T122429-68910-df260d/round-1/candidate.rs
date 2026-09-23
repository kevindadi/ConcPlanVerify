use std::sync::{Arc, Mutex, Condvar};
use std::thread;

struct Handshake {
    // Number of workers that have arrived at the handshake point.
    arrived: Mutex<usize>,
    // Signaled when both workers have arrived.
    ready: Condvar,
}

impl Handshake {
    fn new() -> Self {
        Handshake {
            arrived: Mutex::new(0),
            ready: Condvar::new(),
        }
    }

    fn wait(&self) {
        let mut arrived = self.arrived.lock().unwrap();
        *arrived += 1;
        if *arrived == 2 {
            self.ready.notify_all();
        } else {
            while *arrived < 2 {
                arrived = self.ready.wait(arrived).unwrap();
            }
        }
    }
}

fn main() {
    let m1 = Arc::new(Mutex::new(()));
    let m2 = Arc::new(Mutex::new(()));
    let handshake = Arc::new(Handshake::new());

    let a_m1 = Arc::clone(&m1);
    let a_m2 = Arc::clone(&m2);
    let a_hs = Arc::clone(&handshake);

    let b_m1 = Arc::clone(&m1);
    let b_m2 = Arc::clone(&m2);
    let b_hs = Arc::clone(&handshake);

    let worker_a = thread::spawn(move || {
        let _g1 = a_m1.lock().unwrap();
        a_hs.wait();
        let _g2 = a_m2.lock().unwrap();
    });

    let worker_b = thread::spawn(move || {
        let _g2 = b_m2.lock().unwrap();
        b_hs.wait();
        let _g1 = b_m1.lock().unwrap();
    });

    let bystander = thread::spawn(|| loop {
        // Independent task that keeps making progress forever.
        std::hint::spin_loop();
    });

    worker_a.join().unwrap();
    worker_b.join().unwrap();

    // The bystander runs forever; detach it by not joining.
    drop(bystander);
}
