mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};

use std::thread;

fn s(m: &Mutex<()>, ch1_tx: SyncSender<i64>, ch2_rx: Receiver<i64>) {
    let guard = m.lock().unwrap();
    drop(guard);
    ch1_tx.send(1).unwrap();
    let ack: i64 = ch2_rx.recv().unwrap();
    let _ = ack;
}

fn r(m: &Mutex<()>, ch1_rx: Receiver<i64>, ch2_tx: SyncSender<i64>) {
    let guard = m.lock().unwrap();
    drop(guard);
    let v: i64 = ch1_rx.recv().unwrap();
    let _ = v;
    ch2_tx.send(1).unwrap();
}

fn main() { cir_trace::init();
    let m: Mutex<()> = Mutex::new_named("res_mutex0", ());
    let ch1 = sync_channel::<i64>(0);
    let ch2 = sync_channel::<i64>(0);
    let (ch1_tx, ch1_rx) = ch1;
    let (ch2_tx, ch2_rx) = ch2;
    let mut done: i64 = 0;

    thread::scope(|scope| {
        scope.spawn(|| s(&m, ch1_tx, ch2_rx));
        scope.spawn(|| r(&m, ch1_rx, ch2_tx));
    });

    let guard = m.lock().unwrap();
    done = 1;
    drop(guard);

    println!("DONE done={}", done);
 cir_trace::finish();}
