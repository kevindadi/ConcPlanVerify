mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{mpsc, Arc};
use std::thread;

fn sender(tx: mpsc::SyncSender<i32>, m: Arc<Mutex<()>>) {
    {
        let _guard = m.lock().unwrap();
    }
    cir_trace::record("channel_send", "tx"); tx.send(1).unwrap();
    {
        let _guard = m.lock().unwrap();
    }
    cir_trace::record("channel_send", "tx"); tx.send(2).unwrap();
}

fn receiver(rx: mpsc::Receiver<i32>, m: Arc<Mutex<()>>) {
    {
        let _guard = m.lock().unwrap();
    }
    cir_trace::record("channel_recv", "rx"); let _ = rx.recv().unwrap();
    {
        let _guard = m.lock().unwrap();
    }
    cir_trace::record("channel_recv", "rx"); let _ = rx.recv().unwrap();
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", ()));
    let (tx, rx) = mpsc::sync_channel::<i32>(1);

    let m_sender = Arc::clone(&m);
    let sender_handle = cir_trace::spawn("sender", move || sender(tx, m_sender));

    let m_receiver = Arc::clone(&m);
    let receiver_handle = cir_trace::spawn("receiver", move || receiver(rx, m_receiver));

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
