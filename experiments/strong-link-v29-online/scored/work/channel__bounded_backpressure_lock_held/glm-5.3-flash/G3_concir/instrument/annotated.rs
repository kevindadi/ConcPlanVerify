mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::sync::mpsc::sync_channel;
use std::thread;

fn sender(ch_tx: std::sync::mpsc::SyncSender<i32>, m: Arc<Mutex<bool>>) {
    // channel_send { channel: main::ch, value: 1 }
    crate::cir_trace::record("channel_send", "ch_tx"); ch_tx.send(1).unwrap();
    // channel_send { channel: main::ch, value: 2 }
    // (blocks while the one-slot channel is full, i.e. until the
    //  receiver has taken the first value)
    crate::cir_trace::record("channel_send", "ch_tx"); ch_tx.send(2).unwrap();
    // mutex_lock { resource: main::m }
    let guard = m.lock().unwrap();
    // read_shared { resource: main::done }
    let _ = *guard;
    // mutex_unlock { resource: main::m }
    drop(guard);
}

fn receiver(ch_rx: std::sync::mpsc::Receiver<i32>, m: Arc<Mutex<bool>>) {
    // channel_recv { channel: main::ch, dst: _ }
    crate::cir_trace::record("channel_recv", "ch_rx"); let _ = ch_rx.recv().unwrap();
    // channel_recv { channel: main::ch, dst: _ }
    // (blocks while the channel is empty)
    crate::cir_trace::record("channel_recv", "ch_rx"); let _ = ch_rx.recv().unwrap();
    // mutex_lock { resource: main::m }
    let mut guard = m.lock().unwrap();
    // write_shared { resource: main::done, expr: true }
    *guard = true;
    // mutex_unlock { resource: main::m }
    drop(guard);
}

fn main() { crate::cir_trace::init();
    // Resource: done (var, protected by m) stored inside the mutex m.
    let done: Arc<Mutex<bool>> = Arc::new(Mutex::new_named("done_mutex0#1263", false));
    // Resource: ch (channel, capacity 1).
    let (ch_tx, ch_rx) = sync_channel::<i32>(1);

    let m_sender = Arc::clone(&done);
    let t_sender = crate::cir_trace::spawn("sender#1426", move || sender(ch_tx, m_sender));

    let m_receiver = Arc::clone(&done);
    let t_receiver = crate::cir_trace::spawn("receiver#1536", move || receiver(ch_rx, m_receiver));

    t_sender.join().unwrap();
    t_receiver.join().unwrap();

    // Read shared state only to print the required terminal line.
    let guard = done.lock().unwrap();
    let flag = if *guard { 1 } else { 0 };
    drop(guard);
    println!("DONE done={}", flag);
 crate::cir_trace::finish();}
