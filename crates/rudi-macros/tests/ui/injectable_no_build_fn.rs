use rudi::injectable;

struct NoBuild;

#[injectable]
impl NoBuild {
    fn other() {}
}

fn main() {}
