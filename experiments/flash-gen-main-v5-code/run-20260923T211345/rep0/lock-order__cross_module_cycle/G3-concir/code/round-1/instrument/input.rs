use std::sync::{Arc, Mutex};
use std::thread;

struct Resources {
    a: Mutex<()>,
    b: Mutex<()>,
}

fn t1(res: Arc<Resources>) {
    {
        let _ga = res.a.lock().unwrap();
        {
            let _gb = res.b.lock().unwrap();
        }
    }
}

fn t2(res: Arc<Resources>) {
    {
        let _ga = res.a.lock().unwrap();
        {
            let _gb = res.b.lock().unwrap();
        }
    }
}

fn main() {
    let res = Arc::new(Resources {
        a: Mutex::new(()),
        b: Mutex::new(()),
    });

    let res1 = Arc::clone(&res);
    let res2 = Arc::clone(&res);

    let h1 = thread::spawn(move || t1(res1));
    let h2 = thread::spawn(move || t2(res2));

    h1.join().unwrap();
    h2.join().unwrap();

    println!("DONE done=1");
}
