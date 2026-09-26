mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", ()));
    let ch = sync_channel::<i32>(1);

    let m_sender = Arc::clone(&m);
    let ch_tx = ch.0;
    let sender_thread = cir_trace::spawn("sender", move || {
        sender(m_sender, ch_tx);
    });

    let m_receiver = Arc::clone(&m);
    let ch_rx = ch.1;
    let receiver_thread = cir_trace::spawn("receiver", move || {
        receiver(m_receiver, ch_rx);
    });

    sender_thread.join().unwrap();
    receiver_thread.join().unwrap();
    println!("DONE done=1");
 cir_trace::finish();}

fn sender(m: Arc<Mutex<()>>, ch: SyncSender<i32>) {
    let guard = m.lock().unwrap();
    drop(guard);
    cir_trace::record("channel_send", "ch"); ch.send(1).unwrap();
    let guard = m.lock().unwrap();
    drop(guard);
    cir_trace::record("channel_send", "ch"); ch.send(2).unwrap();
}

fn receiver(m: Arc<Mutex<()>>, ch: Receiver<i32>) {
    let mut v: i32 = 0;
    let guard = m.lock().unwrap();
    drop(guard);
    cir_trace::record("channel_recv", "ch"); v = ch.recv().unwrap();
    let guard = m.lock().unwrap();
    drop(guard);
    cir_trace::record("channel_recv", "ch"); v = ch.recv().unwrap();
    let _ = v;
}
