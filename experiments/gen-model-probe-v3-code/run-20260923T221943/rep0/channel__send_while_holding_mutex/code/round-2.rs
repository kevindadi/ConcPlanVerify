use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::Mutex;
use std::thread;

fn s(m: &Mutex<i64>, ch1: SyncSender<i64>, ch2_rx: Receiver<i64>) {
    {
        let _guard = m.lock().unwrap();
    }
    ch1.send(1).unwrap();
    let ack = ch2_rx.recv().unwrap();
    let _ = ack;
}

fn r(m: &Mutex<i64>, ch1_rx: Receiver<i64>, ch2: SyncSender<i64>) {
    {
        let _guard = m.lock().unwrap();
    }
    let v = ch1_rx.recv().unwrap();
    let _ = v;
    ch2.send(1).unwrap();
}

fn main() {
    let m = Mutex::new(0i64);
    let (ch1, ch1_rx) = sync_channel::<i64>(0);
    let (ch2, ch2_rx) = sync_channel::<i64>(0);

    thread::scope(|scope| {
        let hs = scope.spawn(|| s(&m, ch1, ch2_rx));
        let hr = scope.spawn(|| r(&m, ch1_rx, ch2));
        hs.join().unwrap();
        hr.join().unwrap();
    });

    {
        let mut done = m.lock().unwrap();
        *done = 1;
    }

    let done = *m.lock().unwrap();
    println!("DONE done={}", done);
}
