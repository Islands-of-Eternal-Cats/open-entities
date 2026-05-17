#[test]
fn register_component_outside_define_registered_components() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/register_component_standalone.rs");
}
