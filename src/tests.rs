use super::*;
use display::format_string;
use std::fs::File;
use std::io::Write;
use std::panic;
use std::path::PathBuf;
use std::process::Command;
use tempfile::Builder;
use tempfile::TempDir;

#[test]
pub fn test_main() {
    assert_cli::Assert::main_binary()
        .with_args(&["src/test_dir"])
        .stdout()
        .is(main_output(true))
        .unwrap();
}

#[test]
pub fn test_main_long_paths() {
    assert_cli::Assert::main_binary()
        .with_args(&["-p", "src/test_dir"])
        .stdout()
        .is(main_output(false))
        .unwrap();
}

#[test]
pub fn test_main_multi_arg() {
    assert_cli::Assert::main_binary()
        .with_args(&["src/test_dir/many/", "src/test_dir/", "src/test_dir"])
        .stdout()
        .is(main_output(true))
        .unwrap();
}

#[cfg(target_os = "macos")]
fn main_output(short_paths: bool) -> String {
    format!(
        "{}
{}
{}
{}",
        format_string("src/test_dir", true, short_paths, " 4.0K", "─┬"),
        format_string("src/test_dir/many", true, short_paths, " 4.0K", " └─┬",),
        format_string(
            "src/test_dir/many/hello_file",
            true,
            short_paths,
            " 4.0K",
            "   ├──",
        ),
        format_string(
            "src/test_dir/many/a_file",
            false,
            short_paths,
            "   0B",
            "   └──",
        ),
    )
}

#[cfg(target_os = "linux")]
fn main_output(short_paths: bool) -> String {
    format!(
        "{}
{}
{}
{}",
        format_string("src/test_dir", true, short_paths, "  12K", "─┬"),
        format_string("src/test_dir/many", true, short_paths, " 8.0K", " └─┬",),
        format_string(
            "src/test_dir/many/hello_file",
            true,
            short_paths,
            " 4.0K",
            "   ├──",
        ),
        format_string(
            "src/test_dir/many/a_file",
            false,
            short_paths,
            "   0B",
            "   └──",
        ),
    )
}

#[test]
pub fn test_apparent_size() {
    let r = format!(
        "{}",
        format_string(
            "src/test_dir/many/hello_file",
            true,
            true,
            "   6B",
            "   ├──",
        ),
    );

    assert_cli::Assert::main_binary()
        .with_args(&["-s", "src/test_dir"])
        .stdout()
        .contains(r)
        .unwrap();
}

#[test]
pub fn test_reverse_flag() {
    // variable names the same length make the output easier to read
    let a = "    ┌── a_file";
    let b = "    ├── hello_file";
    let c = "  ┌─┴ many";
    let d = " ─┴ test_dir";

    assert_cli::Assert::main_binary()
        .with_args(&["-r", "src/test_dir"])
        .stdout()
        .contains(a)
        .stdout()
        .contains(b)
        .stdout()
        .contains(c)
        .stdout()
        .contains(d)
        .unwrap();
}

#[test]
pub fn test_d_flag_works() {
    // We should see the top level directory but not the sub dirs / files:
    assert_cli::Assert::main_binary()
        .with_args(&["-d", "1", "-s", "src/test_dir"])
        .stdout()
        .doesnt_contain("hello_file")
        .unwrap();
}

fn build_temp_file(dir: &TempDir) -> PathBuf {
    let file_path = dir.path().join("notes.txt");
    let mut file = File::create(&file_path).unwrap();
    writeln!(file, "I am a temp file").unwrap();
    file_path
}

#[test]
pub fn test_soft_sym_link() {
    let dir = Builder::new().tempdir().unwrap();
    let file = build_temp_file(&dir);
    let dir_s = dir.path().to_str().unwrap();
    let file_path_s = file.to_str().unwrap();

    let link_name = dir.path().join("the_link");
    let link_name_s = link_name.to_str().unwrap();
    let c = Command::new("ln")
        .arg("-s")
        .arg(file_path_s)
        .arg(link_name_s)
        .output();
    assert!(c.is_ok());

    let r = soft_sym_link_output(dir_s, file_path_s, link_name_s);

    // We cannot guarantee which version will appear first.
    // TODO: Consider adding predictable iteration order (sort file entries by name?)
    assert_cli::Assert::main_binary()
        .with_args(&[dir_s])
        .stdout()
        .contains(r)
        .unwrap();
}

#[cfg(target_os = "macos")]
fn soft_sym_link_output(dir: &str, file_path: &str, link_name: &str) -> String {
    format!(
        "{}
{}
{}",
        format_string(dir, true, true, " 8.0K", "─┬"),
        format_string(file_path, true, true, " 4.0K", " ├──",),
        format_string(link_name, false, true, " 4.0K", " └──",),
    )
}

#[cfg(target_os = "linux")]
fn soft_sym_link_output(dir: &str, file_path: &str, link_name: &str) -> String {
    format!(
        "{}
{}
{}",
        format_string(dir, true, true, " 8.0K", "─┬"),
        format_string(file_path, true, true, " 4.0K", " ├──",),
        format_string(link_name, false, true, "   0B", " └──",),
    )
}

// Hard links are ignored as the inode is the same as the file
#[test]
pub fn test_hard_sym_link() {
    let dir = Builder::new().tempdir().unwrap();
    let file = build_temp_file(&dir);
    let dir_s = dir.path().to_str().unwrap();
    let file_path_s = file.to_str().unwrap();

    let link_name = dir.path().join("the_link");
    let link_name_s = link_name.to_str().unwrap();
    let c = Command::new("ln")
        .arg(file_path_s)
        .arg(link_name_s)
        .output();
    assert!(c.is_ok());

    let (r, r2) = hard_link_output(dir_s, file_path_s, link_name_s);

    // Because this is a hard link the file and hard link look identical. Therefore
    // we cannot guarantee which version will appear first.
    // TODO: Consider adding predictable iteration order (sort file entries by name?)
    let result = panic::catch_unwind(|| {
        assert_cli::Assert::main_binary()
            .with_args(&[dir_s])
            .stdout()
            .contains(r)
            .unwrap();
    });
    if result.is_err() {
        assert_cli::Assert::main_binary()
            .with_args(&[dir_s])
            .stdout()
            .contains(r2)
            .unwrap();
    }
}

#[cfg(target_os = "macos")]
fn hard_link_output(dir_s: &str, file_path_s: &str, link_name_s: &str) -> (String, String) {
    let r = format!(
        "{}
{}",
        format_string(dir_s, true, true, " 4.0K", "─┬"),
        format_string(file_path_s, true, true, " 4.0K", " └──")
    );
    let r2 = format!(
        "{}
{}",
        format_string(dir_s, true, true, " 4.0K", "─┬"),
        format_string(link_name_s, true, true, " 4.0K", " └──")
    );
    (r, r2)
}

#[cfg(target_os = "linux")]
fn hard_link_output(dir_s: &str, file_path_s: &str, link_name_s: &str) -> (String, String) {
    let r = format!(
        "{}
{}",
        format_string(dir_s, true, true, " 8.0K", "─┬"),
        format_string(file_path_s, true, true, " 4.0K", " └──")
    );
    let r2 = format!(
        "{}
{}",
        format_string(dir_s, true, true, " 8.0K", "─┬"),
        format_string(link_name_s, true, true, " 4.0K", " └──")
    );
    (r, r2)
}

//Check we don't recurse down an infinite symlink tree
#[test]
pub fn test_recursive_sym_link() {
    let dir = Builder::new().tempdir().unwrap();
    let dir_s = dir.path().to_str().unwrap();

    let link_name = dir.path().join("the_link");
    let link_name_s = link_name.to_str().unwrap();

    let c = Command::new("ln")
        .arg("-s")
        .arg(dir_s)
        .arg(link_name_s)
        .output();
    assert!(c.is_ok());

    assert_cli::Assert::main_binary()
        .with_args(&[dir_s])
        .stdout()
        .contains(recursive_sym_link_output(dir_s, link_name_s))
        .unwrap();
}

#[cfg(target_os = "macos")]
fn recursive_sym_link_output(dir: &str, link_name: &str) -> String {
    format!(
        "{}
{}",
        format_string(dir, true, true, " 4.0K", "─┬"),
        format_string(link_name, true, true, " 4.0K", " └──",),
    )
}
#[cfg(target_os = "linux")]
fn recursive_sym_link_output(dir: &str, link_name: &str) -> String {
    format!(
        "{}
{}",
        format_string(dir, true, true, " 4.0K", "─┬"),
        format_string(link_name, true, true, "   0B", " └──",),
    )
}

// Check against directories and files whos names are substrings of each other
#[test]
#[cfg(target_os = "macos")]
pub fn test_substring_of_names() {
    assert_cli::Assert::main_binary()
        .with_args(&["src/test_dir2"])
        .stdout()
        .contains(" ─┬ test_dir2")
        .stdout()
        .contains("  ├─┬ dir")
        .stdout()
        .contains("  │ └── hello")
        .stdout()
        .contains("  ├── dir_name_clash")
        .stdout()
        .contains("  └─┬ dir_substring")
        .stdout()
        .contains("    └── hello")
        .unwrap();
}

// Check against directories and files whos names are substrings of each other
#[test]
#[cfg(target_os = "linux")]
pub fn test_substring_of_names() {
    assert_cli::Assert::main_binary()
        .with_args(&["src/test_dir2"])
        .stdout()
        .contains(" ─┬ test_dir2")
        .stdout()
        .contains("  ├─┬ dir")
        .stdout()
        .contains("  │ └── hello")
        .stdout()
        .contains("  ├─┬ dir_substring")
        .stdout()
        .contains("  │ └── hello")
        .stdout()
        .contains("  └── dir_name_clash")
        .unwrap();
}
