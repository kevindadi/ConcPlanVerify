use std::sync::{Arc, Condvar, Mutex};
use std::thread;

// ch: a channel that holds at most one value (capacity 1),
// implemented as a slot plus condition variable.
struct Ch {
    slot: Mutex<Option<i32>>,
    avail: Condvar, // signalled when the slot goes non-empty
    space: Condvar, // signalled when the slot goes empty
}

fn send(ch: &Arc<Ch>, v: i32) {
    let mut slot = ch.slot.lock().unwrap();
    // R4/R5: wait for space WITHOUT holding m (and the slot lock is
    // released inside wait()).
    while slot.is_some() {
        slot = ch.space.wait(slot).unwrap();
    }
    *slot = Some(v);
    drop(slot);
    ch.avail.notify_one();
}

fn recv(ch: &Arc<Ch>) -> i32 {
    let mut slot = ch.slot.lock().unwrap();
    // R4/R5: wait for data WITHOUT holding m.
    while slot.is_none() {
        slot = ch.avail.wait(slot).unwrap();
    }
    let v = slot.take().unwrap();
    drop(slot);
    ch.space.notify_one();
    v
}

fn main() {
    let ch = Arc::new(Ch {
        slot: Mutex::new(None),
        avail: Condvar::new(),
        space: Condvar::new(),
    });
    // m: shared lock both roles occasionally need.
    let m: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));

    let sender = {
        let ch = Arc::clone(&ch);
        let m = Arc::clone(&m);
        thread::spawn(move || {
            for v in 1..=2 {
                send(&ch, v); // R7: second send blocks until first value is taken
                let mut cnt = m.lock().unwrap();
                *cnt += 1;
                drop(cnt); // R5: never hold m across a channel wait
            }
        })
    };

    let receiver = {
        let ch = Arc::clone(&ch);
        let m = Arc::clone(&m);
        thread::spawn(move || {
            // R3/R6: take exactly two values, then finish.
            for _ in 0..2 {
                let _v = recv(&ch);
                let mut cnt = m.lock().unwrap();
                *cnt += 1;
                drop(cnt); // R5: never hold m across a channel wait
            }
        })
    };

    sender.join().expect("sender thread");
    receiver.join().expect("receiver thread");

    let cnt = m.lock().unwrap();
    assert_eq!(*cnt, 2); // both roles finished after two values passed (R3)
    drop(cnt);
    println!("DONE done=1"); // R8
}
