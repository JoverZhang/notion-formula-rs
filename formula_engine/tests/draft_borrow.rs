#[test]
fn public_drafts_share_an_engine_borrow_and_cannot_escape_or_mutate_it() {
    let tests = trybuild::TestCases::new();
    tests.pass("tests/draft_borrow/multiple.rs");
    tests.compile_fail("tests/draft_borrow/upsert.rs");
    tests.compile_fail("tests/draft_borrow/remove.rs");
    tests.compile_fail("tests/draft_borrow/escape.rs");
}
