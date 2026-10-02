mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc};

fn sender(m: Arc<Mutex<()>>, ch: SyncSender<i32>) {
    {
        let _g = m.lock().unwrap();
    }
    cir_trace::record("channel_send", "ch"); ch.send(1).unwrap();
    {
        let _g = m.lock().unwrap();
    }
    cir_trace::record("channel_send", "ch"); ch.send(2).unwrap();
}

fn receiver(m: Arc<Mutex<()>>, ch: Receiver<i32>) {
    cir_trace::record("channel_recv", "ch"); let v1 = ch.recv().unwrap();
    {
        let _g = m.lock().unwrap();
    }
    cir_trace::record("channel_recv", "ch"); let v2 = ch.recv().unwrap();
    {
        let _g = m.lock().unwrap();
    }
    let _ = (v1, v2);
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#569", ()));
    let (ch_tx, ch_rx) = sync_channel::<i32>(1);

    let m_sender = Arc::clone(&m);
    let h_sender = cir_trace::spawn("sender#683", move || sender(m_sender, ch_tx));

    let m_receiver = Arc::clone(&m);
    let h_receiver = cir_trace::spawn("receiver#795", move || receiver(m_receiver, ch_rx));

    h_sender.join().unwrap();
    h_receiver.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
