mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc};
use std::thread;

fn sender(m: Arc<Mutex<()>>, tx: SyncSender<i32>) {
    let guard = m.lock().unwrap();
    drop(guard);

    crate::cir_trace::record("channel_send", "tx"); tx.send(1).unwrap();
    crate::cir_trace::record("channel_send", "tx"); tx.send(2).unwrap();
}

fn receiver(m: Arc<Mutex<()>>, rx: Receiver<i32>) {
    crate::cir_trace::record("channel_recv", "rx"); let first = rx.recv().unwrap();

    let guard = m.lock().unwrap();
    drop(guard);

    crate::cir_trace::record("channel_recv", "rx"); let second = rx.recv().unwrap();
    let _ = (first, second);
}

fn main() { crate::cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#515", ()));
    let (tx, rx) = sync_channel(1);

    let sender_m = Arc::clone(&m);
    let sender_thread = crate::cir_trace::spawn("sender#621", move || sender(sender_m, tx));
    let receiver_thread = crate::cir_trace::spawn("receiver#692", move || receiver(m, rx));

    sender_thread.join().unwrap();
    receiver_thread.join().unwrap();

    println!("DONE done=1");
 crate::cir_trace::finish();}
