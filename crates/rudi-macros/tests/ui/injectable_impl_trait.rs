use rudi::injectable;

trait Foo {}
struct Bar;

#[injectable]
impl Foo for Bar {}

fn main() {}
