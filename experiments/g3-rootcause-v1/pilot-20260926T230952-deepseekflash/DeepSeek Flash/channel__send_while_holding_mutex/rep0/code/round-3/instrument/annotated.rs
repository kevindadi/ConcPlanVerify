mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc};
use std::thread;

fn s(ch1: Sender<i32>, ch2: Receiver<i32>, m: Arc<Mutex<()>>) {
    {
        let _guard = m.lock().unwrap();
    }
    cir_trace::record("channel_send", "ch1"); ch1.send(1).unwrap();
    cir_trace::record("channel_recv", "ch2"); let _ = ch2.recv().unwrap();
}

fn r(ch1: Receiver<i32>, ch2: Sender<i32>, m: Arc<Mutex<()>>) {
    {
        let _guard = m.lock().unwrap();
    }
    cir_trace::record("channel_recv", "ch1"); let _ = ch1.recv().unwrap();
    cir_trace::record("channel_send", "ch2"); ch2.send(1).unwrap();
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0", ()));
    let (ch1_tx, ch1_rx) = channel::<i32>();
    let (ch2_tx, ch2_rx) = channel::<i32>();

    let m_s = Arc::clone(&m);
    let m_r = Arc::clone(&m);

    let hs = cir_trace::spawn("s", move || s(ch1_tx, ch2_rx, m_s));
    let hr = cir_trace::spawn("r", move || r(ch1_rx, ch2_tx, m_r));

    hs.join().unwrap();
    hr.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
