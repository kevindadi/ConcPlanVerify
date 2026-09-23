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

        pub fn acquire(&self) -> Permit<'_> {
            let mut count = self.count.lock().unwrap();
            while *count == 0 {
                count = self.cv.wait(count).unwrap();
            }
            *count -= 1;
            Permit { sem: self }
        }

        pub fn try_acquire(&self) -> Option<Permit<'_>> {
            let mut count = self.count.lock().unwrap();
            if *count == 0 {
                None
            } else {
                *count -= 1;
                Some(Permit { sem: self })
            }
        }

        pub fn release(&self) {
            let mut count = self.count.lock().unwrap();
            *count += 1;
            self.cv.notify_one();
        }
    }

    pub struct Permit<'a> {
        sem: &'a Semaphore,
    }

    impl<'a> Drop for Permit<'a> {
        fn drop(&mut self) {
            self.sem.release();
        }
    }
}

use std::sync::mpsc::{sync_channel, SyncSender, Receiver};
use std::sync::{Arc};
use std::thread;

struct Shared {
    m: Mutex<()>,
    ch: SyncSender<i32>,
}

fn main() { cir_trace::init();
    let (tx, rx): (SyncSender<i32>, Receiver<i32>) = sync_channel(1);
    let shared = Arc::new(Shared {
        m: Mutex::new_named("shared_mutex0", ()),
        ch: tx,
    });

    let shared_sender = Arc::clone(&shared);
    let sender = cir_trace::spawn("sender", move || {
        sender(shared_sender);
    });

    let shared_receiver = Arc::clone(&shared);
    let receiver = cir_trace::spawn("receiver", move || {
        receiver(shared_receiver, rx);
    });

    sender.join().unwrap();
    receiver.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}

fn sender(shared: Arc<Shared>) {
    {
        let _guard = shared.m.lock().unwrap();
    }
    shared.ch.send(1).unwrap();
    {
        let _guard = shared.m.lock().unwrap();
    }
    shared.ch.send(2).unwrap();
}

fn receiver(shared: Arc<Shared>, rx: Receiver<i32>) {
    let _x: i32;
    let _y: i32;
    {
        let _guard = shared.m.lock().unwrap();
    }
    _x = rx.recv().unwrap();
    {
        let _guard = shared.m.lock().unwrap();
    }
    _y = rx.recv().unwrap();
}
