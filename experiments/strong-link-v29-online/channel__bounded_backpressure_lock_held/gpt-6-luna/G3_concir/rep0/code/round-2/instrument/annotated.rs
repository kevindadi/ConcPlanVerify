mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
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
    {
        let _guard = m.lock().unwrap();
    }
    crate::cir_trace::record("channel_recv", "ch_rx"); let _ = ch_rx.recv().unwrap();

    {
        let _guard = m.lock().unwrap();
    }
    crate::cir_trace::record("channel_recv", "ch_rx"); let _ = ch_rx.recv().unwrap();
}

fn main() { crate::cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#598", ()));
    let (ch_tx, ch_rx) = sync_channel(1);

    let sender_m = Arc::clone(&m);
    let sender_handle = crate::cir_trace::spawn("sender#710", move || sender(sender_m, ch_tx));
    let receiver_handle = crate::cir_trace::spawn("receiver#784", move || receiver(m, ch_rx));

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
 crate::cir_trace::finish();}
