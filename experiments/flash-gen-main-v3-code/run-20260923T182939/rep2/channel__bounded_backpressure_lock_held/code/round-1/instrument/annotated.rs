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

        pub fn acquire(&self) -> SemaphorePermit<'_> {
            let mut count = self.count.lock().unwrap();
            while *count == 0 {
                count = self.cv.wait(count).unwrap();
            }
            *count -= 1;
            SemaphorePermit { sem: self }
        }

        pub fn try_acquire(&self) -> Option<SemaphorePermit<'_>> {
            let mut count = self.count.lock().unwrap();
            if *count == 0 {
                None
            } else {
                *count -= 1;
                Some(SemaphorePermit { sem: self })
            }
        }

        pub fn release(&self) {
            let mut count = self.count.lock().unwrap();
            *count += 1;
            self.cv.notify_one();
        }
    }

    pub struct SemaphorePermit<'a> {
        sem: &'a Semaphore,
    }

    impl<'a> Drop for SemaphorePermit<'a> {
        fn drop(&mut self) {
            self.sem.release();
        }
    }
}

use std::sync::mpsc::{sync_channel, SyncSender, Receiver};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", ()));
    let (tx, rx): (SyncSender<i32>, Receiver<i32>) = sync_channel(1);

    let m_sender = Arc::clone(&m);
    let m_receiver = Arc::clone(&m);

    let sender_handle = cir_trace::spawn("sender", move || {
        sender(tx, m_sender);
    });

    let receiver_handle = cir_trace::spawn("receiver", move || {
        receiver(rx, m_receiver);
    });

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}

fn sender(ch: SyncSender<i32>, m: Arc<Mutex<()>>) {
    {
        let _guard = m.lock().unwrap();
    }
    ch.send(1).unwrap();
    {
        let _guard = m.lock().unwrap();
    }
    ch.send(2).unwrap();
}

fn receiver(ch: Receiver<i32>, m: Arc<Mutex<()>>) {
    {
        let _guard = m.lock().unwrap();
    }
    let _x = ch.recv().unwrap();
    {
        let _guard = m.lock().unwrap();
    }
    let _y = ch.recv().unwrap();
}
