use std::path::PathBuf;

use trycmd::TestCases;

#[test]
fn examples() {
    let cargo: PathBuf = std::env::var_os("CARGO")
        .map(Into::into)
        .unwrap_or_else(|| "cargo".into());

    TestCases::new()
        .register_bin("cargo", cargo)
        .case("examples/output/*.md");
}
