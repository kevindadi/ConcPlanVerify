mod r#main {
    use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
    use std::sync::{Arc, Mutex};

    pub fn main() {
        let lk = Arc::new(Mutex::new(()));
        let (ch1, ch1_rx) = sync_channel::<i32>(0);
        let (ch2_tx, ch2) = sync_channel::<i32>(0);

        let lk_r = Arc::clone(&lk);
        std::thread::scope(|scope| {
            scope.spawn(move || s(lk, ch1, ch2));
            scope.spawn(move || r(lk_r, ch1_rx, ch2_tx));
        })
        .unwrap();

        let done = 1;
        println!("DONE done={}", done);
    }

    fn s(lk: Arc<Mutex<()>>, ch1: SyncSender<i32>, ch2: Receiver<i32>) {
        {
            let _g = lk.lock().unwrap();
        }
        ch1.send(1).unwrap();
        let mut v = 0;
        v = ch2.recv().unwrap();
        let _ = v;
        {
            let _g = lk.lock().unwrap();
        }
    }

    fn r(lk: Arc<Mutex<()>>, ch1: Receiver<i32>, ch2: SyncSender<i32>) {
        {
            let _g = lk.lock().unwrap();
        }
        let mut v = 0;
        v = ch1.recv().unwrap();
        let _ = v;
        ch2.send(1).unwrap();
        {
            let _g = lk.lock().unwrap();
        }
    }
}

fn main() {
    r#main::main();
}
