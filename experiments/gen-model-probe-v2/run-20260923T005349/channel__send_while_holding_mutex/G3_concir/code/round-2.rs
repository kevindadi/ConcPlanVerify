use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::Mutex;
use std::thread;

fn s(m: &Mutex<()>, ch1: &SyncSender<i64>, ch2: &Receiver<i64>) {
    let guard = m.lock().unwrap();
    drop(guard);
    ch1.send(1).unwrap();
    let ack = ch2.recv().unwrap();
    let _ = ack;
}

fn r(m: &Mutex<()>, ch1: &Receiver<i64>, ch2: &SyncSender<i64>) {
    let guard = m.lock().unwrap();
    drop(guard);
    let v = ch1.recv().unwrap();
    let _ = v;
    ch2.send(1).unwrap();
}

fn main() {
    let m = Mutex::new(());
    let mut done: i64 = 0;

    let (ch1_tx, ch1_rx) = sync_channel::<i64>(0);
    let (ch2_tx, ch2_rx) = sync_channel::<i64>(0);

    thread::scope(|scope| {
        scope.spawn(|| s(&m, &ch1_tx, &ch2_rx));
        scope.spawn(|| r(&m, &ch1_rx, &ch2_tx));
    });

    let guard = m.lock().unwrap();
    done = 1;
    drop(guard);

    println!("DONE done={}", done);
}
