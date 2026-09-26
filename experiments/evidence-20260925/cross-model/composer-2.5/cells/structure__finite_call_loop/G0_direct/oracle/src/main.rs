mod cir_trace;
use concir_sync::Semaphore;
use std::thread;

fn helper() {
}

fn main() { cir_trace::init();
    let sem = Semaphore::new_named("sem_semaphore0", 1);

    {
        let _permit = sem.acquire();
        thread::spawn(helper).join().unwrap();
    }

    {
        let _permit = sem.acquire();
        thread::spawn(helper).join().unwrap();
    }

    println!("DONE done=1");
 cir_trace::finish();}
