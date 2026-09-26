use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use concir_sync::Semaphore;

struct State {
    sends: u32,
    recvs: u32,
    sum: i32,
}

/// Capacity-1 channel. `P` is the semaphore permit that occupies the single slot.
struct Chan<P> {
    slot: Mutex<Option<(i32, P)>>,
    cv: Condvar,
}

impl<P> Chan<P> {
    fn new(_marker: &P) -> Self {
        Self {
            slot: Mutex::new(None),
            cv: Condvar::new(),
        }
    }
}

fn sender<P: Send>(m: &Mutex<State>, ch: &Chan<P>, slots: &Arc<Semaphore>) {
    for v in [1, 2] {
        // Channel is full until the receiver drops the previous permit.
        // Wait here, before taking `m`.
        let permit = slots.acquire();
        {
            let mut guard = m.lock().unwrap();
            guard.sends += 1;
        }
        {
            let mut guard = ch.slot.lock().unwrap();
            *guard = Some((v, permit));
            ch.cv.notify_one();
        }
    }
}

fn receiver<P: Send>(m: &Mutex<State>, ch: &Chan<P>) {
    for _ in 0..2 {
        let (v, permit) = {
            let mut guard = ch.slot.lock().unwrap();
            while guard.is_none() {
                // Empty channel: wait without holding `m`.
                guard = ch.cv.wait(guard).unwrap();
            }
            guard.take().unwrap()
        };
        // Drop releases the slot exactly once so the sender may pass the next value.
        drop(permit);
        {
            let mut guard = m.lock().unwrap();
            guard.recvs += 1;
            guard.sum += v;
        }
    }
}

fn main() {
    let m = Arc::new(Mutex::new(State {
        sends: 0,
        recvs: 0,
        sum: 0,
    }));
    let slots = Semaphore::new(1);

    thread::scope(|scope| {
        let marker = slots.acquire();
        let ch = Arc::new(Chan::new(&marker));
        marker.release();

        let m_s = Arc::clone(&m);
        let ch_s = Arc::clone(&ch);
        let slots_s = Arc::clone(&slots);
        let sender_handle = scope.spawn(move || sender(&m_s, &ch_s, &slots_s));

        let m_r = Arc::clone(&m);
        let ch_r = Arc::clone(&ch);
        let receiver_handle = scope.spawn(move || receiver(&m_r, &ch_r));

        sender_handle.join().unwrap();
        receiver_handle.join().unwrap();
    });

    let guard = m.lock().unwrap();
    let _ = (guard.sends, guard.recvs, guard.sum);
    println!("DONE done=1");
}
