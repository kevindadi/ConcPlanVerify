use std::sync::{Mutex, mpsc};
use std::thread;

fn main() {
    let m = Mutex::new(());
    let (tx, rx) = mpsc::sync_channel::<i32>(1);

    thread::scope(|scope| {
        scope.spawn(|| sender(&m, &tx));
        scope.spawn(|| receiver(&m, &rx));
    });

    println!("DONE done=1");
}

fn sender(m: &Mutex<()>, ch: &mpsc::SyncSender<i32>) {
    {
        let _guard = m.lock().unwrap();
    }
    ch.send(1).unwrap();
    {
        let _guard = m.lock().unwrap();
    }
    ch.send(2).unwrap();
}

fn receiver(m: &Mutex<()>, ch: &mpsc::Receiver<i32>) {
    let mut v: i32 = 0;
    {
        let _guard = m.lock().unwrap();
    }
    v = ch.recv().unwrap();
    {
        let _guard = m.lock().unwrap();
    }
    v = ch.recv().unwrap();
    let _ = v;
}
