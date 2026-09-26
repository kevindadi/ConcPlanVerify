mod cir_trace;

mod r#main {
    use cir_trace::sync::{channel, Mutex, Receiver, Sender};

    pub fn main() {
        let lk = Mutex::new("main::lk", ());
        let (ch1_tx, ch1_rx) = channel::<i32>("main::ch1", 0);
        let (ch2_tx, ch2_rx) = channel::<i32>("main::ch2", 0);

        let lk_r = lk.clone();
        cir_trace::scope(
            ["main::s", "main::r"],
            move || s(lk, ch1_tx, ch2_rx),
            move || r(lk_r, ch1_rx, ch2_tx),
        );

        let done = 1;
        println!("DONE done={}", done);
    }

    fn s(lk: Mutex<()>, ch1: Sender<i32>, ch2: Receiver<i32>) {
        {
            let _g = lk.lock();
        }
        ch1.send(1);
        let mut v = 0;
        v = ch2.recv();
        let _ = v;
        {
            let _g = lk.lock();
        }
    }

    fn r(lk: Mutex<()>, ch1: Receiver<i32>, ch2: Sender<i32>) {
        {
            let _g = lk.lock();
        }
        let mut v = 0;
        v = ch1.recv();
        let _ = v;
        ch2.send(1);
        {
            let _g = lk.lock();
        }
    }
}

fn main() {
    r#main::main();
}
