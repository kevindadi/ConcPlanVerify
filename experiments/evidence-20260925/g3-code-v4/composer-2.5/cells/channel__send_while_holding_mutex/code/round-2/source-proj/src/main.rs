use std::sync::mpsc;
use std::sync::{Arc, Mutex};

fn main() {
    let lk = Arc::new(Mutex::new(()));
    let (ch1_tx, ch1_rx) = mpsc::sync_channel::<i32>(0);
    let (ch2_tx, ch2_rx) = mpsc::sync_channel::<i32>(0);

    let lk_s = Arc::clone(&lk);
    let handle_s = std::thread::spawn(move || s(lk_s, ch1_tx, ch2_rx));

    let lk_r = Arc::clone(&lk);
    let handle_r = std::thread::spawn(move || r(lk_r, ch1_rx, ch2_tx));

    handle_s.join().unwrap();
    handle_r.join().unwrap();

    println!("DONE done=1");
}

fn s(lk: Arc<Mutex<()>>, ch1: mpsc::SyncSender<i32>, ch2: mpsc::Receiver<i32>) {
    let mut v = 0i32;
    {
        let _guard = lk.lock().unwrap();
    }
    ch1.send(1).unwrap();
    v = ch2.recv().unwrap();
    let _ = v;
    {
        let _guard = lk.lock().unwrap();
    }
}

fn r(lk: Arc<Mutex<()>>, ch1: mpsc::Receiver<i32>, ch2: mpsc::SyncSender<i32>) {
    let mut v = 0i32;
    {
        let _guard = lk.lock().unwrap();
    }
    v = ch1.recv().unwrap();
    ch2.send(1).unwrap();
    let _ = v;
    {
        let _guard = lk.lock().unwrap();
    }
}
