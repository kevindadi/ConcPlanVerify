mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc::{Receiver, SyncSender};
use std::sync::{Arc};
use std::thread;

fn sender(m: Arc<Mutex<()>>, ch: SyncSender<i32>) {
    let guard = m.lock().unwrap();
    drop(guard);
    cir_trace::record("channel_send", "ch"); ch.send(1).unwrap();

    let guard = m.lock().unwrap();
    drop(guard);
    cir_trace::record("channel_send", "ch"); ch.send(2).unwrap();
}

fn receiver(m: Arc<Mutex<()>>, ch: Receiver<i32>) {
    let guard = m.lock().unwrap();
    drop(guard);
    cir_trace::record("channel_recv", "ch"); let first = ch.recv().unwrap();

    let guard = m.lock().unwrap();
    drop(guard);
    cir_trace::record("channel_recv", "ch"); let second = ch.recv().unwrap();

    let _ = (first, second);
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#605", ()));
    let (tx, rx) = std::sync::mpsc::sync_channel(1);

    let sender_m = Arc::clone(&m);
    let sender_thread = cir_trace::spawn("sender#728", move || sender(sender_m, tx));

    let receiver_m = Arc::clone(&m);
    let receiver_thread = cir_trace::spawn("receiver#837", move || receiver(receiver_m, rx));

    sender_thread.join().unwrap();
    receiver_thread.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
