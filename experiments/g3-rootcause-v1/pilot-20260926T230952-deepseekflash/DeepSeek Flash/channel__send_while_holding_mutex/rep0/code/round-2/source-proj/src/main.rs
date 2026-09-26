use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

fn s(tx1: Sender<i32>, rx2: Receiver<i32>, m: Arc<Mutex<()>>) {
    {
        let _guard = m.lock().unwrap();
    }
    tx1.send(1).unwrap();
    let _ = rx2.recv().unwrap();
}

fn r(rx1: Receiver<i32>, tx2: Sender<i32>, m: Arc<Mutex<()>>) {
    {
        let _guard = m.lock().unwrap();
    }
    let _ = rx1.recv().unwrap();
    tx2.send(1).unwrap();
}

fn main() {
    let m = Arc::new(Mutex::new(()));
    let (tx1, rx1) = channel::<i32>();
    let (tx2, rx2) = channel::<i32>();

    let m_s = Arc::clone(&m);
    let m_r = Arc::clone(&m);

    let hs = thread::spawn(move || s(tx1, rx2, m_s));
    let hr = thread::spawn(move || r(rx1, tx2, m_r));

    hs.join().unwrap();
    hr.join().unwrap();

    println!("DONE done=1");
}
