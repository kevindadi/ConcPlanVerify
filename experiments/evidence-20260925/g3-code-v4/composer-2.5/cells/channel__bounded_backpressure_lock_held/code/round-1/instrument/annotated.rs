mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc;
use std::sync::{Arc};
use std::thread;

fn sender(ch: mpsc::SyncSender<i32>, m: Arc<Mutex<()>>) {
    cir_trace::record("channel_send", "ch"); ch.send(1).unwrap();
    let _guard = m.lock().unwrap();
    drop(_guard);
    cir_trace::record("channel_send", "ch"); ch.send(2).unwrap();
}

fn receiver(ch: mpsc::Receiver<i32>, m: Arc<Mutex<()>>) {
    cir_trace::record("channel_recv", "ch"); let v1 = ch.recv().unwrap();
    let _ = v1;
    let _guard = m.lock().unwrap();
    drop(_guard);
    cir_trace::record("channel_recv", "ch"); let v2 = ch.recv().unwrap();
    let _ = v2;
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", ()));
    let (ch_tx, ch_rx) = mpsc::sync_channel::<i32>(1);

    let m_sender = Arc::clone(&m);
    let ch_sender = ch_tx;
    let sender_handle = cir_trace::spawn("sender", move || sender(ch_sender, m_sender));

    let m_receiver = Arc::clone(&m);
    let ch_receiver = ch_rx;
    let receiver_handle = cir_trace::spawn("receiver", move || receiver(ch_receiver, m_receiver));

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
