mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread;

fn s(m: Arc<Mutex<()>>, ch1: SyncSender<i32>, ch2: Receiver<i32>) {
    let mut tmp = 0i32;
    {
        let _guard = m.lock().unwrap();
        tmp = 1;
    }
    cir_trace::record("channel_send", "ch1"); ch1.send(tmp).unwrap();
    cir_trace::record("channel_recv", "ch2"); let _ack = ch2.recv().unwrap();
}

fn r(m: Arc<Mutex<()>>, ch1: Receiver<i32>, ch2: SyncSender<i32>) {
    let mut tmp = 0i32;
    {
        let _guard = m.lock().unwrap();
        tmp = 2;
    }
    let _ = tmp;
    cir_trace::record("channel_recv", "ch1"); let _val = ch1.recv().unwrap();
    cir_trace::record("channel_send", "ch2"); ch2.send(1).unwrap();
}

fn main() { cir_trace::init();
    let m = Arc::new(Mutex::new_named("m_mutex0#617", ()));
    let (ch1_tx, ch1_rx) = sync_channel::<i32>(0);
    let (ch2_tx, ch2_rx) = sync_channel::<i32>(0);

    let m_s = Arc::clone(&m);
    let h_s = cir_trace::spawn("s#774", move || s(m_s, ch1_tx, ch2_rx));

    let m_r = Arc::clone(&m);
    let h_r = cir_trace::spawn("r#866", move || r(m_r, ch1_rx, ch2_tx));

    h_s.join().unwrap();
    h_r.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
