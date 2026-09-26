fn helper() -> i32 {
    let mut done = 0;
    done = 1;
    done
}

fn main() {
    let done = helper();
    let _ = helper();
    println!("DONE done={}", done);
}
