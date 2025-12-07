use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

/// SWE-bench instance structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweBenchInstance {
    pub instance_id: String,
    pub repo: String,
    pub base_commit: String,
    pub problem_statement: String,
    pub patch: Option<String>,
    pub test_patch: Option<String>,
    pub environment_setup_commit: Option<String>,
    pub test_file: Option<String>,
    pub test_command: Option<String>,
    pub hints_text: Option<String>,
    pub created_at: Option<String>,
}

/// Result of running a SWE-bench instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweBenchResult {
    pub instance_id: String,
    pub success: bool,
    pub test_passed: bool,
    pub error_message: Option<String>,
    pub patch_applied: bool,
    pub test_output: Option<String>,
}

/// Load a SWE-bench instance from a JSON file
pub fn load_instance<P: AsRef<Path>>(path: P) -> Result<SweBenchInstance, Box<dyn Error>> {
    let content = fs::read_to_string(path)?;
    let instance: SweBenchInstance = serde_json::from_str(&content)?;
    Ok(instance)
}

/// Load all instances from a directory
pub fn load_instances_from_dir<P: AsRef<Path>>(
    dir: P,
) -> Result<Vec<SweBenchInstance>, Box<dyn Error>> {
    let mut instances = Vec::new();
    let dir = dir.as_ref();

    if !dir.exists() {
        return Ok(instances);
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
            match load_instance(&path) {
                Ok(instance) => instances.push(instance),
                Err(e) => eprintln!("Failed to load instance from {:?}: {}", path, e),
            }
        }
    }

    Ok(instances)
}

/// Apply a patch to a repository
pub fn apply_patch<P: AsRef<Path>>(
    repo_path: P,
    patch: &str,
) -> Result<bool, Box<dyn Error>> {
    let repo_path = repo_path.as_ref();

    // Write patch to a temporary file
    let patch_file = repo_path.join(".swe_bench_patch.patch");
    fs::write(&patch_file, patch)?;

    // Apply patch using git apply
    let output = Command::new("git")
        .arg("apply")
        .arg("--check")
        .arg(&patch_file)
        .current_dir(repo_path)
        .output()?;

    if !output.status.success() {
        fs::remove_file(&patch_file).ok();
        return Ok(false);
    }

    // Actually apply the patch
    let output = Command::new("git")
        .arg("apply")
        .arg(&patch_file)
        .current_dir(repo_path)
        .output()?;

    fs::remove_file(&patch_file).ok();

    Ok(output.status.success())
}

/// Run tests for a SWE-bench instance
pub fn run_tests<P: AsRef<Path>>(
    repo_path: P,
    test_command: Option<&str>,
    test_file: Option<&str>,
) -> Result<(bool, String), Box<dyn Error>> {
    let repo_path = repo_path.as_ref();

    // If test_command is provided, use it
    if let Some(cmd) = test_command {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() {
            return Ok((false, "Empty test command".to_string()));
        }

        let mut command = Command::new(parts[0]);
        command.current_dir(repo_path);
        command.args(&parts[1..]);
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        let output = command.output()?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let combined = format!("STDOUT:\n{}\n\nSTDERR:\n{}", stdout, stderr);

        return Ok((output.status.success(), combined));
    }

    // Otherwise, try to run pytest on the test file
    if let Some(file) = test_file {
        let output = Command::new("pytest")
            .arg(file)
            .arg("-v")
            .current_dir(repo_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let combined = format!("STDOUT:\n{}\n\nSTDERR:\n{}", stdout, stderr);

        return Ok((output.status.success(), combined));
    }

    // Try to run pytest on the entire test directory
    let output = Command::new("pytest")
        .arg("-v")
        .current_dir(repo_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let combined = format!("STDOUT:\n{}\n\nSTDERR:\n{}", stdout, stderr);

    Ok((output.status.success(), combined))
}

/// Check correctness using swe.py script
pub fn check_correctness_with_swe_py<P: AsRef<Path>>(
    swe_py_path: P,
    instance_id: &str,
    test_passed: bool,
    test_output: &str,
) -> Result<bool, Box<dyn Error>> {
    let swe_py_path = swe_py_path.as_ref();

    if !swe_py_path.exists() {
        // If swe.py doesn't exist, just return the test result
        return Ok(test_passed);
    }

    // Create a temporary results file
    let results = serde_json::json!({
        "instance_id": instance_id,
        "test_passed": test_passed,
        "test_output": test_output,
    });

    let results_file = std::env::temp_dir().join(format!("swe_result_{}.json", instance_id));
    fs::write(&results_file, serde_json::to_string_pretty(&results)?)?;

    // Run swe.py script
    let output = Command::new("python")
        .arg(swe_py_path)
        .arg("--results")
        .arg(&results_file)
        .output()?;

    // Clean up
    fs::remove_file(&results_file).ok();

    // Parse output to determine correctness
    let stdout = String::from_utf8_lossy(&output.stdout);
    let _stderr = String::from_utf8_lossy(&output.stderr);

    // If swe.py returns success (exit code 0), consider it correct
    // Otherwise, fall back to test_passed
    if output.status.success() {
        // Try to parse JSON output if available
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&stdout) {
            if let Some(correct) = json.get("correct") {
                return Ok(correct.as_bool().unwrap_or(test_passed));
            }
        }
        Ok(true)
    } else {
        // If swe.py fails, use test result as fallback
        Ok(test_passed)
    }
}

/// Run a single SWE-bench instance
pub async fn run_instance<P: AsRef<Path>>(
    instance: &SweBenchInstance,
    repo_path: P,
    swe_py_path: Option<P>,
) -> Result<SweBenchResult, Box<dyn Error>> {
    let repo_path = repo_path.as_ref();

    // Step 1: Apply the patch if available
    let patch_applied = if let Some(ref patch) = instance.patch {
        match apply_patch(repo_path, patch) {
            Ok(true) => true,
            Ok(false) => {
                return Ok(SweBenchResult {
                    instance_id: instance.instance_id.clone(),
                    success: false,
                    test_passed: false,
                    error_message: Some("Failed to apply patch".to_string()),
                    patch_applied: false,
                    test_output: None,
                });
            }
            Err(e) => {
                return Ok(SweBenchResult {
                    instance_id: instance.instance_id.clone(),
                    success: false,
                    test_passed: false,
                    error_message: Some(format!("Error applying patch: {}", e)),
                    patch_applied: false,
                    test_output: None,
                });
            }
        }
    } else {
        false
    };

    // Step 2: Run tests
    let (test_passed, test_output) = match run_tests(
        repo_path,
        instance.test_command.as_deref(),
        instance.test_file.as_deref(),
    ) {
        Ok((passed, output)) => (passed, output),
        Err(e) => {
            return Ok(SweBenchResult {
                instance_id: instance.instance_id.clone(),
                success: false,
                test_passed: false,
                error_message: Some(format!("Error running tests: {}", e)),
                patch_applied,
                test_output: None,
            });
        }
    };

    // Step 3: Check correctness with swe.py if available
    let final_correctness = if let Some(ref swe_py) = swe_py_path {
        match check_correctness_with_swe_py(swe_py, &instance.instance_id, test_passed, &test_output)
        {
            Ok(correct) => correct,
            Err(e) => {
                eprintln!("Warning: swe.py check failed: {}, using test result", e);
                test_passed
            }
        }
    } else {
        test_passed
    };

    Ok(SweBenchResult {
        instance_id: instance.instance_id.clone(),
        success: final_correctness,
        test_passed: final_correctness,
        error_message: if final_correctness {
            None
        } else {
            Some("Tests failed".to_string())
        },
        patch_applied,
        test_output: Some(test_output),
    })
}

/// Run multiple SWE-bench instances sequentially
pub async fn run_instances<P: AsRef<Path>>(
    instances: Vec<SweBenchInstance>,
    repo_base_path: P,
    swe_py_path: Option<P>,
) -> Result<Vec<SweBenchResult>, Box<dyn Error>> {
    let mut results = Vec::new();
    let repo_base_path = repo_base_path.as_ref();

    for (idx, instance) in instances.iter().enumerate() {
        println!(
            "Running instance {}/{}: {}",
            idx + 1,
            instances.len(),
            instance.instance_id
        );

        // Create a unique directory for this instance
        let instance_repo_path = repo_base_path.join(&instance.instance_id);

        // Clone the repository if it doesn't exist
        if !instance_repo_path.exists() {
            // For now, we assume the repo is already cloned
            // In a full implementation, you would clone it here
            eprintln!(
                "Warning: Repository path {} does not exist. Skipping instance {}",
                instance_repo_path.display(),
                instance.instance_id
            );
            results.push(SweBenchResult {
                instance_id: instance.instance_id.clone(),
                success: false,
                test_passed: false,
                error_message: Some("Repository not found".to_string()),
                patch_applied: false,
                test_output: None,
            });
            continue;
        }

        // Checkout the base commit
        let checkout_output = Command::new("git")
            .arg("checkout")
            .arg(&instance.base_commit)
            .current_dir(&instance_repo_path)
            .output()?;

        if !checkout_output.status.success() {
            eprintln!(
                "Warning: Failed to checkout commit {}. Skipping instance {}",
                instance.base_commit, instance.instance_id
            );
            results.push(SweBenchResult {
                instance_id: instance.instance_id.clone(),
                success: false,
                test_passed: false,
                error_message: Some(format!(
                    "Failed to checkout commit: {}",
                    instance.base_commit
                )),
                patch_applied: false,
                test_output: None,
            });
            continue;
        }

        // Run the instance
        let swe_py_ref = swe_py_path.as_ref().map(|p| p.as_ref());
        match run_instance(instance, &instance_repo_path, swe_py_ref).await {
            Ok(result) => {
                println!(
                    "Instance {}: {}",
                    instance.instance_id,
                    if result.success { "PASSED" } else { "FAILED" }
                );
                results.push(result);
            }
            Err(e) => {
                eprintln!("Error running instance {}: {}", instance.instance_id, e);
                results.push(SweBenchResult {
                    instance_id: instance.instance_id.clone(),
                    success: false,
                    test_passed: false,
                    error_message: Some(e.to_string()),
                    patch_applied: false,
                    test_output: None,
                });
            }
        }
    }

    Ok(results)
}

/// Save results to a JSON file
pub fn save_results<P: AsRef<Path>>(
    results: &[SweBenchResult],
    output_path: P,
) -> Result<(), Box<dyn Error>> {
    let json = serde_json::to_string_pretty(results)?;
    fs::write(output_path, json)?;
    Ok(())
}

/// Print summary statistics
pub fn print_summary(results: &[SweBenchResult]) {
    let total = results.len();
    let passed = results.iter().filter(|r| r.success).count();
    let failed = total - passed;

    println!("\n=== SWE-bench Evaluation Summary ===");
    println!("Total instances: {}", total);
    println!("Passed: {} ({:.2}%)", passed, (passed as f64 / total as f64) * 100.0);
    println!("Failed: {} ({:.2}%)", failed, (failed as f64 / total as f64) * 100.0);

    if failed > 0 {
        println!("\nFailed instances:");
        for result in results.iter().filter(|r| !r.success) {
            println!("  - {}: {}", result.instance_id, result.error_message.as_deref().unwrap_or("Unknown error"));
        }
    }
}

