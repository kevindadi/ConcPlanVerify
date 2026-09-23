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
use std::sync::{Arc, Mutex};
use std::thread;

struct Shared {
    m: Mutex<()>,
    ch: Mutex<Option<(SyncSender<i32>, Receiver<i32>)>>,
}

fn main() {
    let (tx, rx) = sync_channel::<i32>(1);

    let shared = Arc::new(Shared {
        m: Mutex::new(()),
        ch: Mutex::new(Some((tx, rx))),
    });

    let shared_sender = Arc::clone(&shared);
    let shared_receiver = Arc::clone(&shared);

    let sender_handle = thread::spawn(move || {
        sender(shared_sender);
    });

    let receiver_handle = thread::spawn(move || {
        receiver(shared_receiver);
    });

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
}

fn sender(shared: Arc<Shared>) {
    let (tx, _rx) = {
        let guard = shared.ch.lock().unwrap();
        let pair = guard.as_ref().unwrap();
        (pair.0.clone(), ())
    };

    {
        let _guard = shared.m.lock().unwrap();
    }

    tx.send(1).unwrap();

    {
        let _guard = shared.m.lock().unwrap();
    }

    tx.send(2).unwrap();
}

fn receiver(shared: Arc<Shared>) {
    let _tx = {
        let guard = shared.ch.lock().unwrap();
        guard.as_ref().unwrap().0.clone()
    };

    let rx = {
        let guard = shared.ch.lock().unwrap();
        let pair = guard.as_ref().unwrap();
        let rx = pair.1.try_recv();
        match rx {
            Ok(_) => None,
            Err(_) => None,
        }
    };
    let _ = rx;

    let x: i32;
    let y: i32;

    {
        let _guard = shared.m.lock().unwrap();
    }

    x = {
        let guard = shared.ch.lock().unwrap();
        let pair = guard.as_ref().unwrap();
        pair.1.recv().unwrap()
    };

    {
        let _guard = shared.m.lock().unwrap();
    }

    y = {
        let guard = shared.ch.lock().unwrap();
        let pair = guard.as_ref().unwrap();
        pair.1.recv().unwrap()
    };

    let _ = (x, y);
}
