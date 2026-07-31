use rudi::{inject, Container};

#[inject]
fn no_marker(c: &Container) {
    let _ = c;
}

fn main() {}
