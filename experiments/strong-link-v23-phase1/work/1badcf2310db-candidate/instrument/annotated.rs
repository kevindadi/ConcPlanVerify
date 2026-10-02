mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;
use concir_sync::Semaphore;

struct Channel<T> {
    slot: Option<T>,
    // semaphore permits: empty slots and filled slots
    empty: Arc<Semaphore>,
    full: Arc<Semaphore>,
}

impl<T> Channel<T> {
    fn new() -> Self {
        Channel {
            slot: None,
            empty: Semaphore::new(1),
            full: Semaphore::new(0),
        }
    }

    fn send(&mut self, value: T) {
        // wait for an empty slot
        let permit = self.empty.acquire();
        self.slot = Some(value);
        // signal that a value is available
        permit.release();
        self.full.acquire().release();
    }

    fn recv(&mut self) -> T {
        // wait for a filled slot
        let permit = self.full.acquire();
        let value = self.slot.take().unwrap();
        permit.release();
        self.empty.acquire().release();
        value
    }
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#957", ()));
    let ch = Arc::new(Mutex::new_named("ch_mutex0#996", Channel::<i32>::new()));

    let m_sender = Arc::clone(&m);
    let ch_sender = Arc::clone(&ch);

    let sender = cir_trace::spawn("sender#1116", move || {
        // send first value
        {
            let _guard = m_sender.lock().unwrap();
        }
        {
            let mut c = ch_sender.lock().unwrap();
            cir_trace::record("channel_send", "c"); c.send(1);
        }
        // send second value
        {
            let _guard = m_sender.lock().unwrap();
        }
        {
            let mut c = ch_sender.lock().unwrap();
            cir_trace::record("channel_send", "c"); c.send(2);
        }
    });

    let m_receiver = Arc::clone(&m);
    let ch_receiver = Arc::clone(&ch);

    let receiver = cir_trace::spawn("receiver#1632", move || {
        // receive first value
        {
            let _guard = m_receiver.lock().unwrap();
        }
        let v1 = {
            let mut c = ch_receiver.lock().unwrap();
            cir_trace::record("channel_recv", "c"); c.recv()
        };
        // receive second value
        {
            let _guard = m_receiver.lock().unwrap();
        }
        let v2 = {
            let mut c = ch_receiver.lock().unwrap();
            cir_trace::record("channel_recv", "c"); c.recv()
        };
        assert_eq!(v1, 1);
        assert_eq!(v2, 2);
    });

    sender.join().unwrap();
    receiver.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
