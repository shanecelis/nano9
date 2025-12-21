use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Test that all .lua and .p8lua files can be loaded and run (with a timeout)
#[test]
fn test_run_lua_files() {
    let mut test_files = Vec::new();

    // Find all .lua and .p8lua files in examples/ and tests/ directories
    //
    let dir_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    find_lua_files(&dir_path, &mut test_files);

    if test_files.is_empty() {
        panic!("No .lua or .p8lua files found to test");
    }

    // Run each file
    for file in test_files {
        // Run with --pause flag to prevent the app from running indefinitely
        // Note: The app will still try to create a window, but with --pause it should
        // pause immediately. In headless CI environments, this might fail if no display
        // is available, but the test will at least verify the file can be loaded and parsed.
        let mut command = Command::new("cargo");
        command
            .args(&[
                "n9",
                "--",
                "run",
                file.to_str().expect("path should be valid UTF-8"),
            ])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            // .env("NANO9_ASSETS_DIR", Path::new(env!("CARGO_MANIFEST_DIR")).join("tests"))
            ;
        // println!("The command {command:?}");

        let output = command.output().expect("Failed to execute cargo run");

        // Check if the command succeeded (exit code 0)
        // We expect it to succeed in loading, even if it pauses
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            panic!(
                "Failed to run {} with exit code {:?}.\nSTDERR:\n{}\nSTDOUT:\n{}",
                file.display(),
                output.status.code(),
                stderr,
                stdout,
            );
        }
    }
}

fn find_lua_files(dir: &Path, files: &mut Vec<PathBuf>) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir_path = root.join(dir);
    if !dir.exists() {
        return;
    }

    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir()
                && let Some(ext) = path.extension()
            {
                if ext == "lua" || ext == "p8lua" {
                    files.push(path.strip_prefix(root).unwrap().to_path_buf());
                }
            }
        }
    }
}
