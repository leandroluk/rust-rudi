use rudi::{injectable, Container};

struct Ambiguous;

#[injectable]
impl Ambiguous {
    fn build(#[container] _c: &Container) -> Self {
        Ambiguous
    }

    fn other(#[container] _c: &Container) -> Self {
        Ambiguous
    }
}

fn main() {}
