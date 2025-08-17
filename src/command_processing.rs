use std::io::{BufReader, Read};
use std::path::Path;
use std::process::{Command, Stdio};

pub fn run_command(command_path: &Path, command: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(command)
        .current_dir(command_path)
        .args(args)
        .output()
        .map_err(|e| format!("Failed to execute command: {e}"))?;

    if output.status.success() {
        let git_output = String::from_utf8_lossy(&output.stdout);
        Ok(git_output.trim().to_string())
    } else {
        let error_message = format!(
            "Command failed to execute\nError:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        Err(error_message)
    }
}

pub fn pipe_commands(
    command_path: &Path,
    cmd1: &str,
    args1: &[&str],
    cmd2: &str,
    args2: &[&str],
) -> Result<String, Box<dyn std::error::Error>> {
    // Create the first command
    let mut first_cmd = Command::new(cmd1);
    first_cmd.current_dir(command_path);
    first_cmd.args(args1);

    // Create the second command
    let mut second_cmd = Command::new(cmd2);
    second_cmd.current_dir(command_path);
    second_cmd.args(args2);

    // Set up the pipe
    first_cmd.stdout(Stdio::piped());
    second_cmd.stdin(Stdio::piped());

    // Start the first command
    let mut first_process = first_cmd.spawn()?;

    // Get the output of the first command
    let first_output = first_process
        .stdout
        .take()
        .expect("Failed to get stdout of first command");

    // Start the second command, using the output of the first as input
    second_cmd.stdin(Stdio::from(first_output));
    let mut second_process = second_cmd.stdout(Stdio::piped()).spawn()?;

    // Read and print the output of the second command
    let mut reader = BufReader::new(
        second_process
            .stdout
            .as_mut()
            .expect("Failed to get stdout of second command"),
    );

    let mut result = String::new();
    reader.read_to_string(&mut result)?;

    // Wait for both processes to finish
    first_process.wait()?;
    second_process.wait()?;

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    #[test]
    fn test_run_command_success() {
        let temp_dir = temp_dir();
        let command_path = temp_dir.as_path();
        let result = run_command(command_path, "cmd", &["/C", "echo", "Hello, world!"]);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "\"Hello, world!\"");
    }

    #[test]
    fn test_run_command_failure() {
        let temp_dir = temp_dir();
        let command_path = temp_dir.as_path();
        let result = run_command(command_path, "nonexistent_command", &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to execute command"));
    }

    #[test]
    fn test_pipe_commands_success() {
        let temp_dir = temp_dir();
        let command_path = temp_dir.as_path();
        let result = pipe_commands(
            command_path,
            "cmd",
            &["/C", "echo", "Hello, world!"],
            "cmd",
            &["/C", "findstr", "Hello"],
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap().trim(), "\"Hello, world!\"");
    }

    #[test]
    fn test_pipe_commands_failure() {
        let temp_dir = temp_dir();
        let command_path = temp_dir.as_path();
        let result = pipe_commands(
            command_path,
            "cmd",
            &["/C", "echo", "Hello, world!"],
            "cmd",
            &["/C", "findstr", "Nonexistent"],
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap().trim(), "");
    }
}
