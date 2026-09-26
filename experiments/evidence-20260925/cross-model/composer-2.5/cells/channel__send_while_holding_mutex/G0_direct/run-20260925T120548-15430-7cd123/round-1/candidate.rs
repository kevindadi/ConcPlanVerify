use concir_sync::Semaphore;
use std::sync::mpsc::sync_channel;
use std::sync::{Arc, Mutex};
use std::thread;

fn s(ch1: std::sync::mpsc::SyncSender<i32>, ch2: std::sync::mpsc::SyncReceiver<i32>, lock: Arc<Mutex<()>>, sem: Arc<Semaphore>) {
    {
        let _permit = sem.acquire();
        {
            let _g = lock.lock().unwrap();
        }
    }
    ch1.send(1).unwrap();
    ch2.recv().unwrap();
}

fn r(ch2: std::sync::mpsc::SyncSender<i32>, ch1: std::sync::mpsc::SyncReceiver<i32>, lock: Arc<Mutex<()>>, sem: Arc<Semaphore>) {
    let v = ch1.recv().unwrap();
    {
        let _permit = sem.acquire();
        {
            let _g = lock.lock().unwrap();
        }
    }
    ch2.send(v).unwrap();
}

fn main() {
    let lock = Arc::new(Mutex::new(()));
    let sem = Semaphore::new(1);

    let (ch1, ch1_end) = sync_channel::<i32>(0);
    let (ch2, ch2_end) = sync_channel::<i32>(0);

    let lock_s = Arc::clone(&lock);
    let sem_s = Arc::clone(&sem);
    let ts = thread::spawn(move || s(ch1, ch2_end, lock_s, sem_s));

    let lock_r = Arc::clone(&lock);
    let sem_r = Arc::clone(&sem);
    let tr = thread::spawn(move || r(ch2, ch1_end, lock_r, sem_r));

    ts.join().unwrap();
    tr.join().unwrap();

    println!("DONE done=1");
}
