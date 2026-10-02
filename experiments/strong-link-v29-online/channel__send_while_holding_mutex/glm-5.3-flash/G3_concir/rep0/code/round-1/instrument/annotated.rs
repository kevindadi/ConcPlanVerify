mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::sync::mpsc::sync_channel;
use std::thread;

fn s(m: &Arc<Mutex<()>>, ch1_tx: &std::sync::mpsc::SyncSender<i32>, ch2_rx: &std::sync::mpsc::Receiver<i32>) {
    let _guard = m.lock().unwrap();
    drop(_guard);
    crate::cir_trace::record("channel_send", "ch1_tx"); ch1_tx.send(1).unwrap();
    crate::cir_trace::record("channel_recv", "ch2_rx"); let _ = ch2_rx.recv().unwrap();
}

fn r(m: &Arc<Mutex<()>>, ch1_rx: &std::sync::mpsc::Receiver<i32>, ch2_tx: &std::sync::mpsc::SyncSender<i32>) {
    let _guard = m.lock().unwrap();
    drop(_guard);
    crate::cir_trace::record("channel_recv", "ch1_rx"); let _ = ch1_rx.recv().unwrap();
    crate::cir_trace::record("channel_send", "ch2_tx"); ch2_tx.send(1).unwrap();
}

fn main() { crate::cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#588", ()));
    let (ch1_tx, ch1_rx) = sync_channel::<i32>(0);
    let (ch2_tx, ch2_rx) = sync_channel::<i32>(0);

    let m_s = Arc::clone(&m);
    let ch1_tx_s = ch1_tx.clone();
    let ch2_rx_s = ch2_rx;
    let handle_s = crate::cir_trace::spawn("s#812", move || {
        s(&m_s, &ch1_tx_s, &ch2_rx_s);
    });

    let m_r = Arc::clone(&m);
    let ch1_rx_r = ch1_rx;
    let ch2_tx_r = ch2_tx;
    let handle_r = crate::cir_trace::spawn("r#987", move || {
        r(&m_r, &ch1_rx_r, &ch2_tx_r);
    });

    handle_s.join().unwrap();
    handle_r.join().unwrap();

    println!("DONE done=1");
 crate::cir_trace::finish();}
