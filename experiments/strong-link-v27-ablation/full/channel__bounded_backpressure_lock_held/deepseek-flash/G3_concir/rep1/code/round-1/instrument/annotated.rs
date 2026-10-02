mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{mpsc::sync_channel, Arc};
use std::thread;

fn sender(m: Arc<Mutex<()>>, ch_tx: std::sync::mpsc::SyncSender<i32>) {
    {
        let _guard = m.lock().unwrap();
    }
    cir_trace::record("channel_send", "ch_tx"); ch_tx.send(1).unwrap();
    cir_trace::record("channel_send", "ch_tx"); ch_tx.send(2).unwrap();
}

fn receiver(m: Arc<Mutex<()>>, ch_rx: std::sync::mpsc::Receiver<i32>) {
    {
        let _guard = m.lock().unwrap();
    }
    cir_trace::record("channel_recv", "ch_rx"); let v1 = ch_rx.recv().unwrap();
    cir_trace::record("channel_recv", "ch_rx"); let v2 = ch_rx.recv().unwrap();
    let _ = (v1, v2);
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#511", ()));
    let (ch_tx, ch_rx) = sync_channel(1);

    let m_sender = Arc::clone(&m);
    let sender_handle = cir_trace::spawn("sender#623", move || sender(m_sender, ch_tx));

    let m_receiver = Arc::clone(&m);
    let receiver_handle = cir_trace::spawn("receiver#735", move || receiver(m_receiver, ch_rx));

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
