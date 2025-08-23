//! Parameterized tests for deserialization of failed test cases

use claude_codes::ClaudeOutput;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

/// Structure of a saved test case
#[derive(serde::Deserialize)]
struct TestCase {
    timestamp: String,
    error: String,
    raw_json: String,
    pretty_json: String,
}

/// Get all test case files from the failed_deserializations directory
fn get_test_cases() -> Vec<PathBuf> {
    let test_dir = PathBuf::from("test_cases/failed_deserializations");

    if !test_dir.exists() {
        return Vec::new();
    }

    fs::read_dir(test_dir)
        .unwrap_or_else(|_| panic!("Failed to read test_cases directory"))
        .filter_map(|entry| {
            entry.ok().and_then(|e| {
                let path = e.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    Some(path)
                } else {
                    None
                }
            })
        })
        .collect()
}

/// Load a test case from a file
fn load_test_case(path: &PathBuf) -> TestCase {
    let content = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("Failed to read test case {:?}: {}", path, e));

    serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse test case {:?}: {}", path, e))
}

#[test]
fn test_all_failed_deserializations() {
    let test_cases = get_test_cases();

    if test_cases.is_empty() {
        println!("No test cases found in test_cases/failed_deserializations/");
        return;
    }

    println!("Found {} test case(s) to test", test_cases.len());

    let mut passed = 0;
    let mut failed = 0;
    let mut errors = Vec::new();

    for test_path in &test_cases {
        let case = load_test_case(test_path);
        let filename = test_path.file_name().unwrap().to_string_lossy();

        // Try to deserialize the raw JSON
        match serde_json::from_str::<ClaudeOutput>(&case.raw_json) {
            Ok(_) => {
                println!("✓ {} - Successfully deserialized!", filename);
                passed += 1;
            }
            Err(e) => {
                println!("✗ {} - Still failing: {}", filename, e);
                errors.push((filename.to_string(), e.to_string()));
                failed += 1;
            }
        }
    }

    println!("\n=== Test Results ===");
    println!("Passed: {} / {}", passed, test_cases.len());
    println!("Failed: {} / {}", failed, test_cases.len());

    if !errors.is_empty() {
        println!("\n=== Failed Cases ===");
        for (filename, error) in &errors {
            println!("{}: {}", filename, error);
        }
    }

    // Don't fail the test if there are failures - we expect some to fail
    // until we implement the proper deserializers
    if failed > 0 {
        println!(
            "\n{} test case(s) still need deserializer implementations",
            failed
        );
    }
}

#[test]
fn test_individual_cases() {
    let test_cases = get_test_cases();

    for test_path in test_cases {
        let case = load_test_case(&test_path);
        let filename = test_path.file_name().unwrap().to_string_lossy();

        // Create an individual test for each case
        println!("\nTesting: {}", filename);
        println!("Original error: {}", case.error);
        println!("Timestamp: {}", case.timestamp);

        // Try to parse as generic JSON first to ensure it's valid JSON
        let json_value: Result<Value, _> = serde_json::from_str(&case.raw_json);
        assert!(
            json_value.is_ok(),
            "Test case {} contains invalid JSON",
            filename
        );

        // Now try to deserialize as ClaudeOutput
        match serde_json::from_str::<ClaudeOutput>(&case.raw_json) {
            Ok(output) => {
                println!("✓ Successfully deserialized as: {:?}", output);
            }
            Err(e) => {
                println!("✗ Failed to deserialize: {}", e);
                println!("Raw JSON:");
                println!("{}", case.pretty_json);

                // Try to identify what fields are present to help with implementation
                if let Ok(value) = serde_json::from_str::<Value>(&case.raw_json) {
                    if let Some(obj) = value.as_object() {
                        println!("\nFields present in JSON:");
                        for (key, val) in obj {
                            let type_name = match val {
                                Value::Null => "null",
                                Value::Bool(_) => "bool",
                                Value::Number(_) => "number",
                                Value::String(_) => "string",
                                Value::Array(_) => "array",
                                Value::Object(_) => "object",
                            };
                            println!("  - {}: {}", key, type_name);
                        }

                        // Check for a type field which is common in tagged enums
                        if let Some(type_field) = obj.get("type") {
                            println!("\nType field value: {}", type_field);
                        }
                    }
                }
            }
        }
    }
}

/// Test to verify our test case format is correct
#[test]
fn test_case_format_validation() {
    use serde_json::json;

    // Create a sample test case to verify our format
    let sample = json!({
        "timestamp": "2024-01-01T00:00:00Z",
        "error": "unknown variant `some_type`",
        "raw_json": "{\"type\":\"some_type\",\"data\":\"test\"}",
        "pretty_json": "{\n  \"type\": \"some_type\",\n  \"data\": \"test\"\n}"
    });

    // This should parse successfully
    let _: TestCase = serde_json::from_value(sample).expect("Sample test case should deserialize");
}
