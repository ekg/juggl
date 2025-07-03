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
juggl <INPUT_FILE> -d <DELIMITER>
```

### Arguments

- `<INPUT_FILE>` - Path to the input file to shuffle
- `-d, --delimiter <DELIMITER>` - Delimiter to split the file on

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

## How It Works

1. The file is memory-mapped for efficient reading
2. The file is scanned for delimiter positions
3. Chunks between delimiters are identified
4. The chunks are randomly shuffled using a secure random number generator
5. The shuffled chunks are written to stdout with delimiters preserved between them

## Performance

`juggl` uses memory-mapped I/O for efficient file reading, making it suitable for large files. The entire file is mapped into memory but only accessed as needed.

## Testing

Run the test suite:

```bash
cargo test
```

## License

[Add your license here]