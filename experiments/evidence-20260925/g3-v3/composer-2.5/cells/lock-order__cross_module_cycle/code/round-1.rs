mod r#main {
    use std::sync::Mutex;
    use std::thread;

    pub static a: Mutex<()> = Mutex::new(());

    pub fn t1() {
        let _ga = a.lock().unwrap();
        let _gb = super::other::b.lock().unwrap();
    }

    pub fn main() {
        thread::scope(|s| {
            s.spawn(t1);
            s.spawn(super::other::t2);
        });
    }
}

mod other {
    use std::sync::Mutex;

    pub static b: Mutex<()> = Mutex::new(());

    pub fn t2() {
        let _ga = super::r#main::a.lock().unwrap();
        let _gb = b.lock().unwrap();
    }
}

fn main() {
    r#main::main();
    println!("DONE done=1");
}
