use rudi::injectable;

struct WrongParam;

#[injectable]
impl WrongParam {
    fn build(x: u32) -> Self {
        let _ = x;
        WrongParam
    }
}

fn main() {}
