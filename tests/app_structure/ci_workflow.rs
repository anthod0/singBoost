#[test]
fn main_push_build_is_gated_by_cargo_package_version_change() {
    let workflow = std::fs::read_to_string(".github/workflows/ci.yml").unwrap();

    assert!(
        workflow.contains("branches:") && workflow.contains("- main"),
        "CI should run on pushes to main"
    );
    assert!(
        workflow.contains("git diff")
            && workflow.contains("github.event.before")
            && workflow.contains("Cargo.toml")
            && workflow.contains("^[+-]version"),
        "Main push builds should check whether Cargo.toml package version changed"
    );
    assert!(
        workflow.contains("needs.version.outputs.should_build")
            && workflow.contains("if: needs.version.outputs.should_build == 'true'")
            && workflow.contains("should_build=false"),
        "Windows build should be skipped on main pushes unless the package version changed"
    );
}
