#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/01-flat_mod.rs");
    t.pass("tests/ui/02-sync_only.rs");
    t.pass("tests/ui/03-async_only.rs");
    t.pass("tests/ui/04-unit_test_util.rs");
    t.pass("tests/ui/05-differentiate_sync_async.rs");
    t.compile_fail("tests/ui/test_fail/01-sync_only.rs");
    t.compile_fail("tests/ui/test_fail/02-async_only.rs");
    t.compile_fail("tests/ui/test_fail/03-amphi_option_gt2.rs");
    t.compile_fail("tests/ui/test_fail/04-amphi_option_both_async_blocking.rs");
    t.compile_fail("tests/ui/test_fail/05-absolute_path.rs");
}
