# juggl

A fast command-line tool for shuffling file chunks based on delimiters.

## Overview

`juggl` reads a file, splits it into chunks based on a specified delimiter, randomly shuffles those chunks, and outputs the result. This is useful for randomizing data sets, shuffling lines in files, or reordering any delimited content.

## Installation

```bash
cargo build --release
```

The binary will be available at `target/release/juggl`.

## Usage

```bash
juggl <INPUT_FILE> -d <DELIMITER> [OPTIONS]
```

### Arguments

- `<INPUT_FILE>` - Path to the input file to shuffle
- `-d, --delimiter <DELIMITER>` - Delimiter to split the file on
- `-s, --seed <SEED>` - Random seed for reproducible shuffling (optional)

### Delimiter Formats

The delimiter supports several formats:

- **Plain text**: `-d ","` or `-d "::"`
- **Escape sequences**:
  - `\n` - newline
  - `\r` - carriage return
  - `\t` - tab
  - `\0` - null byte
  - `\\` - literal backslash
- **Hex values**: `-d "\x00"` or `-d "\x0a"`

## Examples

### Shuffle CSV fields
```bash
juggl data.csv -d "," > shuffled.csv
```

### Shuffle lines in a file
```bash
juggl lines.txt -d "\n" > shuffled_lines.txt
```

### Shuffle null-terminated records
```bash
juggl records.dat -d "\x00" > shuffled_records.dat
```

### Shuffle with multi-character delimiter
```bash
juggl data.txt -d "::" > shuffled.txt
```

### Reproducible shuffle with seed
```bash
juggl data.csv -d "," -s 42 > shuffled.csv
# Running with the same seed produces the same output
juggl data.csv -d "," -s 42 > shuffled2.csv
# shuffled.csv and shuffled2.csv will be identical
```

## How It Works

1. The file is memory-mapped for efficient reading
2. The file is scanned for delimiter positions
3. Chunk boundaries are stored as (start, end) position pairs in a vector
4. The vector of boundaries is shuffled using the Fisher-Yates algorithm (via Rust's `shuffle()` method)
5. The shuffled chunks are written to stdout by reading from the memory-mapped file in the new order

## Performance and Memory Usage

`juggl` uses memory-mapped I/O for efficient file reading, making it suitable for large files. The memory requirements are:

- **File content**: Memory-mapped (not loaded into RAM, paged in as needed by the OS)
- **Chunk boundaries**: Small vector storing position pairs - requires ~16 bytes per chunk
- **Shuffling**: In-place Fisher-Yates shuffle with O(n) time complexity

For example, a 1GB file with 1 million chunks would only need ~16MB of RAM for the boundary data, while the file content remains memory-mapped.

## Testing

Run the test suite:

```bash
cargo test
```

## License

MIT License - see the [LICENSE](LICENSE) file for details.