mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use concir_sync::Semaphore;
use std::sync::mpsc::{sync_channel, SyncSender, Receiver};
use std::sync::{Arc};
use std::thread;

fn main() { cir_trace::init();
    let (tx, rx): (SyncSender<i32>, Receiver<i32>) = sync_channel(1);
    let m = Arc::new(Mutex::new_named("m_mutex0", ()));

    let m_sender = Arc::clone(&m);
    let tx_sender = tx.clone();
    let sender = cir_trace::spawn("sender", move || {
        {
            let _guard = m_sender.lock().unwrap();
        }
        cir_trace::record("channel_send", "tx_sender"); tx_sender.send(1).unwrap();
        {
            let _guard = m_sender.lock().unwrap();
        }
        cir_trace::record("channel_send", "tx_sender"); tx_sender.send(2).unwrap();
    });

    let m_receiver = Arc::clone(&m);
    let receiver = cir_trace::spawn("receiver", move || {
        {
            let _guard = m_receiver.lock().unwrap();
        }
        cir_trace::record("channel_recv", "rx"); let _x = rx.recv().unwrap();
        {
            let _guard = m_receiver.lock().unwrap();
        }
        cir_trace::record("channel_recv", "rx"); let _y = rx.recv().unwrap();
    });

    sender.join().unwrap();
    receiver.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
