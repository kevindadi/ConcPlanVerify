use std::sync::{Arc, Mutex, Condvar};
use std::thread;
use concir_sync::Semaphore;

struct Channel {
    slot: Option<i32>,
    // capacity is 1
}

struct ChanState {
    chan: Mutex<Channel>,
    not_empty: Condvar,
    not_full: Condvar,
}

impl ChanState {
    fn new() -> Self {
        ChanState {
            chan: Mutex::new(Channel { slot: None }),
            not_empty: Condvar::new(),
            not_full: Condvar::new(),
        }
    }

    fn send(&self, v: i32) {
        let mut guard = self.chan.lock().unwrap();
        while guard.slot.is_some() {
            guard = self.not_full.wait(guard).unwrap();
        }
        guard.slot = Some(v);
        self.not_empty.notify_one();
    }

    fn recv(&self) -> i32 {
        let mut guard = self.chan.lock().unwrap();
        while guard.slot.is_none() {
            guard = self.not_empty.wait(guard).unwrap();
        }
        let v = guard.slot.take().unwrap();
        self.not_full.notify_one();
        v
    }
}

fn main() {
    let ch = Arc::new(ChanState::new());
    let m = Arc::new(Mutex::new(0i32));
    // Two permits: one for each role to consume when it finishes.
    let done = Arc::new(Semaphore::new(2));

    let ch_s = Arc::clone(&ch);
    let m_s = Arc::clone(&m);
    let done_s = Arc::clone(&done);

    let sender = thread::spawn(move || {
        // First value: take the shared lock briefly, then release it
        // before touching the channel (R5).
        {
            let _g = m_s.lock().unwrap();
        }
        ch_s.send(1);

        // Second value: again release the lock before sending.
        {
            let _g = m_s.lock().unwrap();
        }
        ch_s.send(2);

        // Signal completion by consuming one permit.
        done_s.acquire();
    });

    let ch_r = Arc::clone(&ch);
    let m_r = Arc::clone(&m);
    let done_r = Arc::clone(&done);

    let receiver = thread::spawn(move || {
        let a = ch_r.recv();
        {
            let _g = m_r.lock().unwrap();
        }
        let b = ch_r.recv();
        {
            let _g = m_r.lock().unwrap();
        }
        assert_eq!(a, 1);
        assert_eq!(b, 2);

        // Signal completion by consuming one permit.
        done_r.acquire();
    });

    // Wait for both roles to finish.
    sender.join().unwrap();
    receiver.join().unwrap();

    println!("DONE done=1");
}
