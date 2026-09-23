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
use std::thread;

fn main() {
    let (tx1, rx1): (SyncSender<i32>, Receiver<i32>) = sync_channel(0);
    let (tx2, rx2): (SyncSender<i32>, Receiver<i32>) = sync_channel(0);

    let s_handle = thread::spawn(move || {
        s(tx1, rx2);
    });

    let r_handle = thread::spawn(move || {
        r(rx1, tx2);
    });

    s_handle.join().unwrap();
    r_handle.join().unwrap();

    println!("DONE done=1");
}

fn s(tx1: SyncSender<i32>, rx2: Receiver<i32>) {
    tx1.send(1).unwrap();
    let _ack = rx2.recv().unwrap();
}

fn r(rx1: Receiver<i32>, tx2: SyncSender<i32>) {
    let _v = rx1.recv().unwrap();
    tx2.send(1).unwrap();
}
