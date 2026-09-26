use concir_sync::Semaphore;
use std::thread;

fn helper() {}

fn main() {
    let sem = Semaphore::new(1);

    {
        let _permit = sem.acquire();
        thread::spawn(helper).join().unwrap();
    }

    {
        let _permit = sem.acquire();
        thread::spawn(helper).join().unwrap();
    }

    println!("DONE done=1");
}
