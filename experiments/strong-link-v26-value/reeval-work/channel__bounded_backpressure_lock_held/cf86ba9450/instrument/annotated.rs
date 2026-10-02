mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
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

fn main() { cir_trace::init();
    let ch = Arc::new(Ch {
        slot: Mutex::new_named("slot#999", None),
        avail: Condvar::new_named("avail#1034"),
        space: Condvar::new_named("space#1065"),
    });
    // m: shared lock both roles occasionally need.
    let m: Arc<Mutex<usize>> = Arc::new(Mutex::new_named("res_mutex0#1179", 0));

    let sender = {
        let ch = Arc::clone(&ch);
        let m = Arc::clone(&m);
        cir_trace::spawn("send#1282", move || {
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
        cir_trace::spawn("recv#1691", move || {
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
 cir_trace::finish();}
