mod concir_sync {
    use std::sync::{Arc, Condvar, Mutex};

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
            let mut c = self.count.lock().unwrap();
            while *c == 0 {
                c = self.cv.wait(c).unwrap();
            }
            *c -= 1;
            Permit { sem: self }
        }

        pub fn try_acquire(&self) -> Option<Permit<'_>> {
            let mut c = self.count.lock().unwrap();
            if *c == 0 {
                None
            } else {
                *c -= 1;
                Some(Permit { sem: self })
            }
        }

        pub fn release(&self) {
            let mut c = self.count.lock().unwrap();
            *c += 1;
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
use std::sync::{Arc, Mutex};
use std::thread;

struct Shared {
    m: Mutex<()>,
    ch: SyncSender<i64>,
}

fn main() {
    let (tx, rx): (SyncSender<i64>, Receiver<i64>) = sync_channel(1);
    let shared = Arc::new(Shared {
        m: Mutex::new(()),
        ch: tx,
    });

    let shared_sender = Arc::clone(&shared);
    let shared_receiver = Arc::clone(&shared);

    let sender = thread::spawn(move || {
        sender(shared_sender);
    });

    let receiver = thread::spawn(move || {
        receiver(shared_receiver, rx);
    });

    sender.join().unwrap();
    receiver.join().unwrap();

    println!("DONE done=1");
}

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

fn receiver(shared: Arc<Shared>, rx: Receiver<i64>) {
    let _x: i64;
    let _y: i64;
    {
        let _guard = shared.m.lock().unwrap();
    }
    _x = rx.recv().unwrap();
    {
        let _guard = shared.m.lock().unwrap();
    }
    _y = rx.recv().unwrap();
}
