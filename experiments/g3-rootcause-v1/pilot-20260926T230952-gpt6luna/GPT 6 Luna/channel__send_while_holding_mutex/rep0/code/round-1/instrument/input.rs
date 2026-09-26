use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread;

struct Shared {
    count: i32,
    done: i32,
}

fn s(m: Arc<Mutex<Shared>>, ch1: SyncSender<i32>, ch2: Receiver<i32>) {
    {
        let mut guard = m.lock().unwrap();
        guard.count += 1;
    }

    ch1.send(1).unwrap();
    let reply = ch2.recv().unwrap();

    {
        let mut guard = m.lock().unwrap();
        guard.count += 1;
    }

    let _ = reply;
}

fn r(m: Arc<Mutex<Shared>>, ch1: Receiver<i32>, ch2: SyncSender<i32>) {
    {
        let mut guard = m.lock().unwrap();
        guard.count += 1;
    }

    let message = ch1.recv().unwrap();
    ch2.send(2).unwrap();

    {
        let mut guard = m.lock().unwrap();
        guard.count += 1;
    }

    let _ = message;
}

fn main() {
    let m = Arc::new(Mutex::new(Shared { count: 0, done: 0 }));
    let (ch1_tx, ch1_rx) = sync_channel(0);
    let (ch2_tx, ch2_rx) = sync_channel(0);

    let s_m = Arc::clone(&m);
    let s_thread = thread::spawn(move || s(s_m, ch1_tx, ch2_rx));

    let r_m = Arc::clone(&m);
    let r_thread = thread::spawn(move || r(r_m, ch1_rx, ch2_tx));

    s_thread.join().unwrap();
    r_thread.join().unwrap();

    {
        let mut guard = m.lock().unwrap();
        guard.done = 1;
    }

    let guard = m.lock().unwrap();
    println!("DONE done={}", guard.done);
}
