mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
mod cir_trace;

mod r#main {
    use cir_trace::sync::{channel, Mutex, Receiver, Sender};

    pub fn main() {
        let lk = Mutex::new_named("lk_mutex0", "main::lk", ());
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
        cir_trace::record("channel_send", "ch1"); ch1.send(1);
        let mut v = 0;
        cir_trace::record("channel_recv", "ch2"); v = ch2.recv();
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
        cir_trace::record("channel_recv", "ch1"); v = ch1.recv();
        let _ = v;
        cir_trace::record("channel_send", "ch2"); ch2.send(1);
        {
            let _g = lk.lock();
        }
    }
}

fn main() { cir_trace::init();
    r#main::main();
 cir_trace::finish();}
