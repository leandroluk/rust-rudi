#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/injectable_*.rs");
    t.compile_fail("tests/ui/inject_*.rs");
    t.compile_fail("tests/ui/derive_*.rs");
}
