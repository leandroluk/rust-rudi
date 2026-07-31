use rudi::injectable;

struct Bad;

#[injectable]
impl Bad {
    fn build(#[inject] x: u32) -> Self {
        let _ = x;
        Bad
    }
}

fn main() {}
