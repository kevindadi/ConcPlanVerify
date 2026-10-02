mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc::{Receiver, SyncSender};
use std::sync::{Arc};
use std::thread;

fn sender(m: Arc<Mutex<()>>, ch_tx: SyncSender<i32>) {
    {
        let _guard = m.lock().unwrap();
    }
    crate::cir_trace::record("channel_send", "ch_tx"); ch_tx.send(1).unwrap();

    {
        let _guard = m.lock().unwrap();
    }
    crate::cir_trace::record("channel_send", "ch_tx"); ch_tx.send(2).unwrap();
}

fn receiver(m: Arc<Mutex<()>>, ch_rx: Receiver<i32>) {
    crate::cir_trace::record("channel_recv", "ch_rx"); let first = ch_rx.recv().unwrap();

    {
        let _guard = m.lock().unwrap();
    }
    crate::cir_trace::record("channel_recv", "ch_rx"); let second = ch_rx.recv().unwrap();

    {
        let _guard = m.lock().unwrap();
    }

    let _ = (first, second);
}

fn main() { crate::cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#624", ()));
    let (ch_tx, ch_rx) = std::sync::mpsc::sync_channel(1);

    let sender_m = Arc::clone(&m);
    let sender_thread = crate::cir_trace::spawn("sender#753", move || sender(sender_m, ch_tx));

    let receiver_thread = crate::cir_trace::spawn("receiver#828", move || receiver(m, ch_rx));

    sender_thread.join().unwrap();
    receiver_thread.join().unwrap();

    let done = 1;
    println!("DONE done={}", done);
 crate::cir_trace::finish();}
