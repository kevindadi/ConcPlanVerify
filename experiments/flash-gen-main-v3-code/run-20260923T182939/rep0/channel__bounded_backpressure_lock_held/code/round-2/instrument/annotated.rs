mod cir_trace;
mod concir_sync;
use cir_trace::sync::{Mutex, Condvar};
mod concir_sync {
    use std::sync::{Arc};

    pub struct Semaphore {
        count: Mutex<usize>,
        cv: Condvar,
    }

    impl Semaphore {
        pub fn new(n: usize) -> Arc<Semaphore> {
            Arc::new(Semaphore {
                count: Mutex::new(n),
                cv: Condvar::new(),
            })
        }

        pub fn acquire(self: &Arc<Self>) -> SemaphorePermit {
            let mut count = self.count.lock().unwrap();
            while *count == 0 {
                count = self.cv.wait(count).unwrap();
            }
            *count -= 1;
            SemaphorePermit {
                sem: Arc::clone(self),
            }
        }

        pub fn try_acquire(self: &Arc<Self>) -> Option<SemaphorePermit> {
            let mut count = self.count.lock().unwrap();
            if *count == 0 {
                None
            } else {
                *count -= 1;
                Some(SemaphorePermit {
                    sem: Arc::clone(self),
                })
            }
        }

        pub fn release(self: &Arc<Self>) {
            let mut count = self.count.lock().unwrap();
            *count += 1;
            self.cv.notify_one();
        }
    }

    pub struct SemaphorePermit {
        sem: Arc<Semaphore>,
    }

    impl Drop for SemaphorePermit {
        fn drop(&mut self) {
            self.sem.release();
        }
    }
}

use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc};
use std::thread;

struct Shared {
    m: Mutex<()>,
    ch: SyncSender<i64>,
}

fn main() { cir_trace::init();
    let (tx, rx): (SyncSender<i64>, Receiver<i64>) = sync_channel(1);

    let shared = Arc::new(Shared {
        m: Mutex::new_named("shared_mutex0", ()),
        ch: tx,
    });

    let shared_sender = Arc::clone(&shared);
    let shared_receiver = Arc::clone(&shared);

    let sender_handle = cir_trace::spawn("sender", move || {
        sender(&shared_sender);
    });

    let receiver_handle = cir_trace::spawn("receiver", move || {
        receiver(&shared_receiver, rx);
    });

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}

fn sender(shared: &Arc<Shared>) {
    {
        let _guard = shared.m.lock().unwrap();
    }
    shared.ch.send(1).unwrap();

    {
        let _guard = shared.m.lock().unwrap();
    }
    shared.ch.send(2).unwrap();
}

fn receiver(shared: &Arc<Shared>, rx: Receiver<i64>) {
    let x: i64;
    {
        let _guard = shared.m.lock().unwrap();
    }
    x = rx.recv().unwrap();

    let y: i64;
    {
        let _guard = shared.m.lock().unwrap();
    }
    y = rx.recv().unwrap();

    let _ = (x, y);
}
