mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread;

fn sender(m: Arc<Mutex<()>>, ch: SyncSender<i32>) {
    {
        let _guard = m.lock().unwrap();
    }
    cir_trace::record("channel_send", "ch"); ch.send(1).unwrap();
    cir_trace::record("channel_send", "ch"); ch.send(2).unwrap();
}

fn receiver(m: Arc<Mutex<()>>, ch: Receiver<i32>) {
    {
        let _guard = m.lock().unwrap();
    }
    cir_trace::record("channel_recv", "ch"); let _ = ch.recv().unwrap();
    cir_trace::record("channel_recv", "ch"); let _ = ch.recv().unwrap();
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", ()));
    let (tx, rx) = sync_channel::<i32>(1);

    let m_sender = Arc::clone(&m);
    let m_receiver = Arc::clone(&m);

    let sender_handle = cir_trace::spawn("sender", move || sender(m_sender, tx));
    let receiver_handle = cir_trace::spawn("receiver", move || receiver(m_receiver, rx));

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
