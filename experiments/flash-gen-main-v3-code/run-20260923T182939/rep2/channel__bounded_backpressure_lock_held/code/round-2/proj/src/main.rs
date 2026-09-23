mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::sync::mpsc::{sync_channel, SyncSender, Receiver};
use std::thread;

struct Channel {
    tx: SyncSender<i32>,
    rx: Mutex<Option<Receiver<i32>>>,
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", ()));
    let (tx, rx) = sync_channel::<i32>(1);
    let ch = Arc::new(Channel {
        tx,
        rx: Mutex::new_named("ch_mutex0", Some(rx)),
    });

    let m_sender = Arc::clone(&m);
    let ch_sender = Arc::clone(&ch);
    let sender = cir_trace::spawn("sender", move || {
        {
            let _g = m_sender.lock().unwrap();
        }
        ch_sender.tx.send(1).unwrap();
        {
            let _g = m_sender.lock().unwrap();
        }
        ch_sender.tx.send(2).unwrap();
    });

    let m_receiver = Arc::clone(&m);
    let ch_receiver = Arc::clone(&ch);
    let receiver = cir_trace::spawn("receiver", move || {
        let rx = ch_receiver.rx.lock().unwrap().take().unwrap();
        {
            let _g = m_receiver.lock().unwrap();
        }
        let _x: i32 = rx.recv().unwrap();
        {
            let _g = m_receiver.lock().unwrap();
        }
        let _y: i32 = rx.recv().unwrap();
    });

    sender.join().unwrap();
    receiver.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
