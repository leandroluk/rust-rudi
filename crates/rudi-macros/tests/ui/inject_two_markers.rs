use rudi::{inject, Container};

#[inject]
fn two_markers(#[container] a: &Container, #[container] b: &Container) {
    let _ = (a, b);
}

fn main() {}
