mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::sync::mpsc::sync_channel;
use std::thread;

fn s(
    ch1_tx: std::sync::mpsc::SyncSender<i32>,
    ch2_rx: std::sync::mpsc::Receiver<i32>,
    m: &Arc<Mutex<i32>>,
) {
    {
        let _guard = m.lock().unwrap();
    }
    crate::cir_trace::record("channel_send", "ch1_tx"); let _ = ch1_tx.send(1);
    crate::cir_trace::record("channel_recv", "ch2_rx"); let got = ch2_rx.recv().unwrap();
    let _ = got;
}

fn r(
    ch1_rx: std::sync::mpsc::Receiver<i32>,
    ch2_tx: std::sync::mpsc::SyncSender<i32>,
    m: &Arc<Mutex<i32>>,
) {
    {
        let _guard = m.lock().unwrap();
    }
    crate::cir_trace::record("channel_recv", "ch1_rx"); let got = ch1_rx.recv().unwrap();
    crate::cir_trace::record("channel_send", "ch2_tx"); let _ = ch2_tx.send(1);
    let _ = got;
}

fn main() { crate::cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#648", 0));

    let (ch1_tx, ch1_rx) = sync_channel::<i32>(0);
    let (ch2_tx, ch2_rx) = sync_channel::<i32>(0);

    let m_s = Arc::clone(&m);
    let h_s = crate::cir_trace::spawn("s#805", move || s(ch1_tx, ch2_rx, &m_s));

    let m_r = Arc::clone(&m);
    let h_r = crate::cir_trace::spawn("r#898", move || r(ch1_rx, ch2_tx, &m_r));

    h_s.join().unwrap();
    h_r.join().unwrap();

    let done = *m.lock().unwrap();
    let _ = done;
    println!("DONE done=1");
 crate::cir_trace::finish();}
