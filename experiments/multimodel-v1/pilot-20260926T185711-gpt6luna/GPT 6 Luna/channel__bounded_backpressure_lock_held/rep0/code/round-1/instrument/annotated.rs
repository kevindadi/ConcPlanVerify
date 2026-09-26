mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc};
use std::thread;

fn sender(ch: SyncSender<i32>, m: Arc<Mutex<()>>) {
    {
        let _guard = m.lock().unwrap();
    }
    cir_trace::record("channel_send", "ch"); ch.send(1).unwrap();

    {
        let _guard = m.lock().unwrap();
    }
    cir_trace::record("channel_send", "ch"); ch.send(2).unwrap();
}

fn receiver(ch: Receiver<i32>, m: Arc<Mutex<()>>) {
    let mut value = 0;
    cir_trace::record("channel_recv", "ch"); value = ch.recv().unwrap();
    {
        let _guard = m.lock().unwrap();
    }

    cir_trace::record("channel_recv", "ch"); value = ch.recv().unwrap();
    {
        let _guard = m.lock().unwrap();
    }
    let _ = value;
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", ()));
    let ch = sync_channel::<i32>(1);
    let (ch_tx, ch_rx) = ch;

    let sender_m = Arc::clone(&m);
    let sender_handle = cir_trace::spawn("sender", move || sender(ch_tx, sender_m));

    let receiver_handle = cir_trace::spawn("receiver", move || receiver(ch_rx, m));

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
