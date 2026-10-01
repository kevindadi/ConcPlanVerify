use std::sync::Arc;

fn helper() {
    // no operations
}

fn main() {
    let mut done = 0;

    helper();
    done = 1;

    println!("DONE done={}", done);
}
