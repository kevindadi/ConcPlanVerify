mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc::{self, SyncSender, Receiver};
use std::sync::{Arc};
use std::thread;

static LOCK: Mutex<()> = Mutex::new(());
static DONE: Mutex<i32> = Mutex::new(0);

fn s(tx: SyncSender<i32>) {
    {
        let _g = LOCK.lock().unwrap();
    }
    tx.send(1).unwrap();
    {
        let _g = LOCK.lock().unwrap();
    }
}

fn r(rx: Receiver<i32>) {
    {
        let _g = LOCK.lock().unwrap();
    }
    let _v = rx.recv().unwrap();
    {
        let _g = LOCK.lock().unwrap();
    }
    {
        let mut d = DONE.lock().unwrap();
        *d = 1;
    }
}

fn main() { cir_trace::init();
    let (tx, rx) = mpsc::sync_channel::<i32>(0);

    let s_handle = cir_trace::spawn("s_handle", move || s(tx));
    let r_handle = cir_trace::spawn("r_handle", move || r(rx));

    s_handle.join().unwrap();
    r_handle.join().unwrap();

    let done = *DONE.lock().unwrap();
    println!("DONE done={}", done);
 cir_trace::finish();}
