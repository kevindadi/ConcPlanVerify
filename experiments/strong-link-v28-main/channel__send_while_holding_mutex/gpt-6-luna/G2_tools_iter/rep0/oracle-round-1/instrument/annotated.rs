mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{mpsc::sync_channel, Arc, Barrier};
use std::thread;

fn s(
    ch1: std::sync::mpsc::SyncSender<u32>,
    ch2: std::sync::mpsc::Receiver<u32>,
    shared: Arc<Mutex<usize>>,
    start: Arc<Barrier>,
) {
    start.wait();

    {
        let mut value = shared.lock().unwrap();
        *value += 1;
    }

    cir_trace::record("channel_send", "ch1"); ch1.send(1).unwrap();
    cir_trace::record("channel_recv", "ch2"); let _reply = ch2.recv().unwrap();

    {
        let mut value = shared.lock().unwrap();
        *value += 1;
    }
}

fn r(
    ch1: std::sync::mpsc::Receiver<u32>,
    ch2: std::sync::mpsc::SyncSender<u32>,
    shared: Arc<Mutex<usize>>,
    start: Arc<Barrier>,
) {
    start.wait();

    {
        let mut value = shared.lock().unwrap();
        *value += 1;
    }

    cir_trace::record("channel_recv", "ch1"); let _message = ch1.recv().unwrap();
    cir_trace::record("channel_send", "ch2"); ch2.send(2).unwrap();

    {
        let mut value = shared.lock().unwrap();
        *value += 1;
    }
}

fn main() { cir_trace::init();
    let (ch1_sender, ch1_receiver) = sync_channel::<u32>(0);
    let (ch2_sender, ch2_receiver) = sync_channel::<u32>(0);
    let shared = Arc::new(Mutex::new_named("shared_mutex0#1045", 0));
    let start = Arc::new(Barrier::new(2));

    let s_shared = Arc::clone(&shared);
    let s_start = Arc::clone(&start);
    let s_handle = cir_trace::spawn("s#1195", move || {
        s(ch1_sender, ch2_receiver, s_shared, s_start);
    });

    let r_shared = Arc::clone(&shared);
    let r_start = Arc::clone(&start);
    let r_handle = cir_trace::spawn("r#1381", move || {
        r(ch1_receiver, ch2_sender, r_shared, r_start);
    });

    s_handle.join().unwrap();
    r_handle.join().unwrap();

    println!("DONE done=1");
 cir_trace::finish();}
