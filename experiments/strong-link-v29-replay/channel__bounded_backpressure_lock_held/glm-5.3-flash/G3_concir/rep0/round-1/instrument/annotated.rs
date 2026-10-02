mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::Arc;
use std::sync::mpsc::sync_channel;
use std::thread;

fn sender(m: Arc<crate::cir_trace::sync::Mutex<i32>>, ch: std::sync::mpsc::SyncSender<i32>) {
    // mutex_lock main::m
    let _guard = m.lock().unwrap();
    // mutex_unlock main::m
    drop(_guard);

    let v1: i32 = 1;
    let v2: i32 = 2;

    // channel_send main::ch v1
    crate::cir_trace::record("channel_send", "ch"); ch.send(v1).unwrap();
    // channel_send main::ch v2 (blocks until receiver has taken v1)
    crate::cir_trace::record("channel_send", "ch"); ch.send(v2).unwrap();
}

fn receiver(m: Arc<crate::cir_trace::sync::Mutex<i32>>, ch: std::sync::mpsc::Receiver<i32>) {
    // mutex_lock main::m
    let _guard = m.lock().unwrap();
    // mutex_unlock main::m
    drop(_guard);

    let mut r1: i32 = 0;
    let mut r2: i32 = 0;

    // channel_recv main::ch -> r1
    crate::cir_trace::record("channel_recv", "ch"); r1 = ch.recv().unwrap();
    // channel_recv main::ch -> r2
    crate::cir_trace::record("channel_recv", "ch"); r2 = ch.recv().unwrap();

    let _ = (r1, r2);
}

fn main() { crate::cir_trace::init();
    // Shared resources: m (lock), ch (channel, capacity 1)
    let m: Arc<crate::cir_trace::sync::Mutex<i32>> = Arc::new(crate::cir_trace::sync::Mutex::new_named("m_mutex0#996", 0));
    let (tx, rx) = sync_channel::<i32>(1);

    let m_sender = Arc::clone(&m);
    let hs = crate::cir_trace::spawn("sender#1097", move || {
        sender(m_sender, tx);
    });

    let m_receiver = Arc::clone(&m);
    let hr = crate::cir_trace::spawn("receiver#1210", move || {
        receiver(m_receiver, rx);
    });

    // join hs
    hs.join().unwrap();
    // join hr
    hr.join().unwrap();

    println!("DONE done=1");
 crate::cir_trace::finish();}
