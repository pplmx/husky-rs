mod common;

use common::TestProject;
use std::io::Error;

#[test]
fn test_helper_file_support() -> Result<(), Error> {
    let project = TestProject::new("helper-test-")?;
    project.add_husky_rs("dependencies", false)?;

    let helper_content = "#!/bin/sh\nhello() {\n  echo \"Hello from helper\"\n}\n";
    project.create_hook("_helpers.sh", helper_content)?;

    let hook_content = r#"#!/bin/sh
. "$(dirname "$0")/_helpers.sh"
hello
"#;
    project.create_hook("pre-commit", hook_content)?;

    project.build()?;

    assert!(project.path.join(".husky").join("_helpers.sh").exists());
    assert!(project.path.join(".husky").join("pre-commit").exists());

    project.assert_hook_installed("pre-commit");

    Ok(())
}
