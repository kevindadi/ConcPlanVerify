mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::mpsc::{self, Receiver, SyncSender};


struct Shared {
    progress: i64,
    done: i64,
    out: String,
}

fn sender(m: &Mutex<Shared>, ch: &SyncSender<i64>) {
    {
        let mut guard = m.lock().unwrap();
        guard.progress += 1;
    }
    ch.send(1).unwrap();
    {
        let mut guard = m.lock().unwrap();
        guard.progress += 1;
    }
    ch.send(2).unwrap();
}

fn receiver(m: &Mutex<Shared>, ch: &Receiver<i64>) {
    let first = ch.recv().unwrap();
    {
        let mut guard = m.lock().unwrap();
        guard.progress += 1;
    }
    let second = ch.recv().unwrap();
    {
        let mut guard = m.lock().unwrap();
        guard.progress += 1;
    }
    let _ = (first, second);
}

fn main() { cir_trace::init();
    let m = Mutex::new_named("m_mutex0", Shared {
        progress: 0,
        done: 0,
        out: String::new(),
    });
    let ch = mpsc::sync_channel::<i64>(1);
    let (ch_tx, ch_rx) = ch;

    std::thread::scope(|s| {
        s.spawn(|| sender(&m, &ch_tx));
        s.spawn(|| receiver(&m, &ch_rx));
    });

    let line = {
        let mut guard = m.lock().unwrap();
        guard.done = 1;
        guard.out = String::from("DONE done=1");
        guard.out.clone()
    };
    println!("{}", line);
 cir_trace::finish();}
