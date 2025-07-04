use std::fs::File;
use std::io::Write;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_basic_shuffle() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("input.txt");
    
    let mut file = File::create(&input_file).unwrap();
    write!(file, "apple,banana,cherry,date").unwrap();
    
    let output = Command::new("cargo")
        .args(&["run", "--", &input_file.to_string_lossy(), "-d", ","])
        .output()
        .expect("Failed to execute command");
    
    assert!(output.status.success());
    let result = String::from_utf8_lossy(&output.stdout);
    
    assert!(result.contains("apple"));
    assert!(result.contains("banana"));
    assert!(result.contains("cherry"));
    assert!(result.contains("date"));
    
    // Count commas to verify we have 4 parts
    let comma_count = result.matches(',').count();
    assert!(comma_count >= 3, "Expected at least 3 commas, found {}", comma_count);
}

#[test]
fn test_newline_delimiter() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("lines.txt");
    
    let mut file = File::create(&input_file).unwrap();
    writeln!(file, "line1").unwrap();
    writeln!(file, "line2").unwrap();
    writeln!(file, "line3").unwrap();
    write!(file, "line4").unwrap();
    
    let output = Command::new("cargo")
        .args(&["run", "--", &input_file.to_string_lossy(), "-d", "\\n"])
        .output()
        .expect("Failed to execute command");
    
    assert!(output.status.success());
    let result = String::from_utf8_lossy(&output.stdout);
    
    assert!(result.contains("line1"));
    assert!(result.contains("line2"));
    assert!(result.contains("line3"));
    assert!(result.contains("line4"));
}

#[test]
fn test_hex_delimiter() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("hex.dat");
    
    let mut file = File::create(&input_file).unwrap();
    file.write_all(b"part1\x00part2\x00part3").unwrap();
    
    let output = Command::new("cargo")
        .args(&["run", "--", &input_file.to_string_lossy(), "-d", "\\x00"])
        .output()
        .expect("Failed to execute command");
    
    assert!(output.status.success());
    let result = output.stdout;
    
    assert!(result.windows(5).any(|w| w == b"part1"));
    assert!(result.windows(5).any(|w| w == b"part2"));
    assert!(result.windows(5).any(|w| w == b"part3"));
}

#[test]
fn test_multi_char_delimiter() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("multi.txt");
    
    let mut file = File::create(&input_file).unwrap();
    write!(file, "foo::bar::baz::qux").unwrap();
    
    let output = Command::new("cargo")
        .args(&["run", "--", &input_file.to_string_lossy(), "-d", "::"])
        .output()
        .expect("Failed to execute command");
    
    assert!(output.status.success());
    let result = String::from_utf8_lossy(&output.stdout);
    
    assert!(result.contains("foo"));
    assert!(result.contains("bar"));
    assert!(result.contains("baz"));
    assert!(result.contains("qux"));
}

#[test]
fn test_empty_file() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("empty.txt");
    
    File::create(&input_file).unwrap();
    
    let output = Command::new("cargo")
        .args(&["run", "--", &input_file.to_string_lossy(), "-d", ","])
        .output()
        .expect("Failed to execute command");
    
    assert!(output.status.success());
    assert_eq!(output.stdout.len(), 0);
}

#[test]
fn test_file_not_found() {
    let output = Command::new("cargo")
        .args(&["run", "--", "nonexistent.txt", "-d", ","])
        .output()
        .expect("Failed to execute command");
    
    assert!(!output.status.success());
}

#[test]
fn test_delimiter_at_start_and_end() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("delim_edges.txt");
    
    let mut file = File::create(&input_file).unwrap();
    write!(file, ",a,b,c,").unwrap();
    
    let output = Command::new("cargo")
        .args(&["run", "--", &input_file.to_string_lossy(), "-d", ","])
        .output()
        .expect("Failed to execute command");
    
    assert!(output.status.success());
    let result = String::from_utf8_lossy(&output.stdout);
    
    assert!(result.contains("a"));
    assert!(result.contains("b"));
    assert!(result.contains("c"));
}

#[test]
fn test_consecutive_delimiters() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("consecutive.txt");
    
    let mut file = File::create(&input_file).unwrap();
    write!(file, "a,,b,,c").unwrap();
    
    let output = Command::new("cargo")
        .args(&["run", "--", &input_file.to_string_lossy(), "-d", ","])
        .output()
        .expect("Failed to execute command");
    
    assert!(output.status.success());
    let result = String::from_utf8_lossy(&output.stdout);
    
    assert!(result.contains("a"));
    assert!(result.contains("b"));
    assert!(result.contains("c"));
}

#[test]
fn test_large_file() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("large.txt");
    
    let mut file = File::create(&input_file).unwrap();
    let chunks: Vec<String> = (0..1000).map(|i| format!("chunk{}", i)).collect();
    write!(file, "{}", chunks.join(",")).unwrap();
    
    let output = Command::new("cargo")
        .args(&["run", "--", &input_file.to_string_lossy(), "-d", ","])
        .output()
        .expect("Failed to execute command");
    
    assert!(output.status.success());
    let result = String::from_utf8_lossy(&output.stdout);
    
    for i in 0..1000 {
        assert!(result.contains(&format!("chunk{}", i)));
    }
}

#[test]
fn test_special_characters_in_content() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("special.txt");
    
    let mut file = File::create(&input_file).unwrap();
    write!(file, "hello\nworld|test\ttab|end").unwrap();
    
    let output = Command::new("cargo")
        .args(&["run", "--", &input_file.to_string_lossy(), "-d", "|"])
        .output()
        .expect("Failed to execute command");
    
    assert!(output.status.success());
    let result = String::from_utf8_lossy(&output.stdout);
    
    assert!(result.contains("hello\nworld"));
    assert!(result.contains("test\ttab"));
    assert!(result.contains("end"));
}

#[test]
fn test_seeded_shuffle_reproducible() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("seeded.txt");
    
    let mut file = File::create(&input_file).unwrap();
    write!(file, "1,2,3,4,5,6,7,8,9,10").unwrap();
    
    // Run with seed 42 twice
    let output1 = Command::new("cargo")
        .args(&["run", "--", &input_file.to_string_lossy(), "-d", ",", "-s", "42"])
        .output()
        .expect("Failed to execute command");
    
    let output2 = Command::new("cargo")
        .args(&["run", "--", &input_file.to_string_lossy(), "-d", ",", "-s", "42"])
        .output()
        .expect("Failed to execute command");
    
    assert!(output1.status.success());
    assert!(output2.status.success());
    
    // Both outputs should be identical
    assert_eq!(output1.stdout, output2.stdout);
}

#[test]
fn test_different_seeds_produce_different_outputs() {
    let temp_dir = TempDir::new().unwrap();
    let input_file = temp_dir.path().join("different_seeds.txt");
    
    let mut file = File::create(&input_file).unwrap();
    write!(file, "a,b,c,d,e,f,g,h,i,j,k,l,m,n,o,p").unwrap();
    
    // Run with different seeds
    let output1 = Command::new("cargo")
        .args(&["run", "--", &input_file.to_string_lossy(), "-d", ",", "-s", "42"])
        .output()
        .expect("Failed to execute command");
    
    let output2 = Command::new("cargo")
        .args(&["run", "--", &input_file.to_string_lossy(), "-d", ",", "-s", "123"])
        .output()
        .expect("Failed to execute command");
    
    assert!(output1.status.success());
    assert!(output2.status.success());
    
    // Outputs should be different (with high probability)
    assert_ne!(output1.stdout, output2.stdout);
    
    // But both should contain all the same elements
    let result1 = String::from_utf8_lossy(&output1.stdout);
    let result2 = String::from_utf8_lossy(&output2.stdout);
    
    for letter in ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p'] {
        assert!(result1.contains(letter));
        assert!(result2.contains(letter));
    }
}