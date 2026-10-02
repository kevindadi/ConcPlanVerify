mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{mpsc::{sync_channel, Receiver, SyncSender}, Arc};
use std::thread;

fn sender(ch: SyncSender<i32>, m: Arc<Mutex<()>>) {
    for value in [1, 2] {
        {
            let _guard = m.lock().unwrap();
        } // Release m before a potentially blocking send.
        cir_trace::record("channel_send", "ch"); ch.send(value).unwrap();
    }
}

fn receiver(ch: Receiver<i32>, m: Arc<Mutex<()>>) {
    for _ in 0..2 {
        {
            let _guard = m.lock().unwrap();
        } // Release m before a potentially blocking receive.
        cir_trace::record("channel_recv", "ch"); ch.recv().unwrap();
    }
}

fn main() { cir_trace::init();
    let ch = sync_channel(1);
    let (ch_tx, ch_rx) = ch;
    let m = Arc::new(Mutex::new_named("m_mutex0#659", ()));

    let sender_handle = {
        let m = Arc::clone(&m);
        cir_trace::spawn("sender#736", move || sender(ch_tx, m))
    };
    let receiver_handle = cir_trace::spawn("receiver#809", move || receiver(ch_rx, m));

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
