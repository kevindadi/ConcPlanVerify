mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc};
use std::thread;

fn sender(ch: SyncSender<i32>, m: Arc<Mutex<()>>) {
    cir_trace::record("channel_send", "ch"); ch.send(1).unwrap();
    let guard = m.lock().unwrap();
    drop(guard);
    cir_trace::record("channel_send", "ch"); ch.send(2).unwrap();
}

fn receiver(ch: Receiver<i32>, m: Arc<Mutex<()>>) {
    let mut a: i32 = 0;
    let mut b: i32 = 0;
    cir_trace::record("channel_recv", "ch"); a = ch.recv().unwrap();
    let guard = m.lock().unwrap();
    drop(guard);
    cir_trace::record("channel_recv", "ch"); b = ch.recv().unwrap();
    let _ = (a, b);
}

fn main() { cir_trace::init();
    let (tx, rx) = sync_channel::<i32>(1);
    let m = Arc::new(Mutex::new_named("m_mutex0", ()));
    let m_receiver = Arc::clone(&m);

    let sender_handle = cir_trace::spawn("sender", move || sender(tx, m));
    let receiver_handle = cir_trace::spawn("receiver", move || receiver(rx, m_receiver));

    sender_handle.join().unwrap();
    receiver_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
