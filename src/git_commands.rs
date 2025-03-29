use crate::command_processing::{pipe_commands, run_command};
use std::path::Path;

pub fn get_commit_tree_hash(repo_path: &Path, commit_hash: &str) -> Result<String, String> {
    let command = "git";
    let args = ["log", "--pretty=format:%T", "-n", "1", commit_hash];

    run_command(repo_path, command, &args)
}

pub fn get_commit_deps(repo_path: &Path, commit_hash: &str) -> Result<String, String> {
    let command = "git";
    let commit_part = format!("{}~1..{}", commit_hash, commit_hash);
    let args = ["rev-list", "--objects", &commit_part];

    run_command(repo_path, command, &args)
}

pub fn get_object_type(repo_path: &Path, hash: &str) -> Result<String, String> {
    let command = "git";
    let args = ["cat-file", "-t", hash];

    run_command(repo_path, command, &args)
}

pub fn get_commit_deps_old(repo_path: &Path, commit_hash: &str) -> Result<String, String> {
    let command = "git";
    let args = ["diff-tree", "--no-commit-id", "-r", commit_hash];

    run_command(repo_path, command, &args)
}

pub fn list_objects(repo_path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let command = "git";
    let rev_list_args = ["rev-list", "--objects", "--all", "--no-object-names"];
    let cat_file_args = [
        "cat-file",
        "--batch-check='%(objecttype) %(objectname) %(objectsize) %(objectsize:disk)'",
    ];

    pipe_commands(repo_path, command, &rev_list_args, command, &cat_file_args)
}

pub fn get_tag_deps(repo_path: &Path) -> Result<String, String> {
    // git show-ref --tags -d
    let command = "git";
    let args = ["show-ref", "--tags", "-d"];

    run_command(repo_path, command, &args)
}
