use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::Mutex;
use std::thread;

fn main() {
    let m = Mutex::new(());
    let (tx, rx) = mpsc::sync_channel::<i32>(1);

    thread::scope(|scope| {
        scope.spawn(|| sender(&m, tx));
        scope.spawn(|| receiver(&m, rx));
    });

    println!("DONE done=1");
}

fn sender(m: &Mutex<()>, ch: SyncSender<i32>) {
    let guard = m.lock().unwrap();
    drop(guard);
    ch.send(1).unwrap();
    let guard = m.lock().unwrap();
    drop(guard);
    ch.send(2).unwrap();
}

fn receiver(m: &Mutex<()>, ch: Receiver<i32>) {
    let mut v: i32 = 0;
    let _ = v;
    let guard = m.lock().unwrap();
    drop(guard);
    v = ch.recv().unwrap();
    let _ = v;
    let guard = m.lock().unwrap();
    drop(guard);
    v = ch.recv().unwrap();
    let _ = v;
}
