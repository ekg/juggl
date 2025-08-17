use clap::Parser;
use memmap2::MmapOptions;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rayon::prelude::*;
use std::fs::File;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(name = "juggl")]
#[command(about = "Shuffles chunks of a file based on delimiters", long_about = None)]
struct Args {
    #[arg(help = "Input file path")]
    input: PathBuf,

    #[arg(short, long, help = "Delimiter (supports escape sequences like \\x00)")]
    delimiter: String,

    #[arg(short, long, help = "Random seed for reproducible shuffling")]
    seed: Option<u64>,

    #[arg(short, long, help = "Number of threads for parallel processing (default: number of CPU cores)")]
    threads: Option<usize>,
}

fn parse_delimiter(delim: &str) -> Vec<u8> {
    let mut result = Vec::new();
    let chars: Vec<char> = delim.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '\\' && i + 1 < chars.len() {
            match chars[i + 1] {
                'n' => {
                    result.push(b'\n');
                    i += 2;
                }
                'r' => {
                    result.push(b'\r');
                    i += 2;
                }
                't' => {
                    result.push(b'\t');
                    i += 2;
                }
                '0' => {
                    result.push(b'\0');
                    i += 2;
                }
                'x' => {
                    if i + 3 < chars.len() {
                        let hex = &delim[i + 2..i + 4];
                        if let Ok(byte) = u8::from_str_radix(hex, 16) {
                            result.push(byte);
                            i += 4;
                        } else {
                            result.push(chars[i] as u8);
                            i += 1;
                        }
                    } else {
                        result.push(chars[i] as u8);
                        i += 1;
                    }
                }
                _ => {
                    result.push(chars[i + 1] as u8);
                    i += 2;
                }
            }
        } else {
            result.push(chars[i] as u8);
            i += 1;
        }
    }

    result
}

fn find_delimiter_positions(data: &[u8], delimiter: &[u8]) -> Vec<usize> {
    if delimiter.is_empty() || data.len() < delimiter.len() {
        return vec![0];
    }

    let delimiter = Arc::new(delimiter.to_vec());
    let data_len = data.len();
    let delim_len = delimiter.len();
    
    let chunk_size = std::cmp::max(1_000_000, data_len / rayon::current_num_threads());
    
    let all_positions: Vec<Vec<usize>> = (0..data_len)
        .into_par_iter()
        .step_by(chunk_size)
        .map(|start| {
            let end = std::cmp::min(start + chunk_size + delim_len - 1, data_len);
            let delimiter = delimiter.clone();
            let mut local_positions = Vec::new();
            
            let mut i = start;
            while i <= end.saturating_sub(delim_len) {
                if &data[i..i + delim_len] == delimiter.as_slice() {
                    local_positions.push(i + delim_len);
                    i += delim_len;
                } else {
                    i += 1;
                }
            }
            
            local_positions
        })
        .collect();
    
    let mut positions = vec![0];
    for chunk_positions in all_positions {
        positions.extend(chunk_positions);
    }
    
    positions.sort_unstable();
    positions.dedup();
    
    positions
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    
    if let Some(threads) = args.threads {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build_global()
            .expect("Failed to set thread count");
    }
    
    let delimiter = parse_delimiter(&args.delimiter);
    
    let file = File::open(&args.input)?;
    let mmap = unsafe { MmapOptions::new().map(&file)? };
    
    let positions = find_delimiter_positions(&mmap, &delimiter);
    
    let mut chunks: Vec<(usize, usize)> = Vec::new();
    for i in 0..positions.len() - 1 {
        chunks.push((positions[i], positions[i + 1]));
    }
    
    if positions.last().copied().unwrap_or(0) < mmap.len() {
        chunks.push((positions.last().copied().unwrap_or(0), mmap.len()));
    }
    
    if let Some(seed) = args.seed {
        let mut rng = StdRng::seed_from_u64(seed);
        chunks.shuffle(&mut rng);
    } else {
        let mut rng = rand::rng();
        chunks.shuffle(&mut rng);
    }
    
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    
    for (i, &(start, end)) in chunks.iter().enumerate() {
        if start < end {
            let chunk_data = &mmap[start..end];
            
            if chunk_data.starts_with(&delimiter) {
                handle.write_all(&chunk_data[delimiter.len()..])?;
            } else {
                handle.write_all(chunk_data)?;
            }
            
            if i < chunks.len() - 1 && !chunk_data.ends_with(&delimiter) {
                handle.write_all(&delimiter)?;
            }
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_delimiter_simple() {
        assert_eq!(parse_delimiter("abc"), vec![b'a', b'b', b'c']);
        assert_eq!(parse_delimiter(","), vec![b',']);
        assert_eq!(parse_delimiter("|"), vec![b'|']);
    }

    #[test]
    fn test_parse_delimiter_escape_sequences() {
        assert_eq!(parse_delimiter("\\n"), vec![b'\n']);
        assert_eq!(parse_delimiter("\\r"), vec![b'\r']);
        assert_eq!(parse_delimiter("\\t"), vec![b'\t']);
        assert_eq!(parse_delimiter("\\0"), vec![b'\0']);
    }

    #[test]
    fn test_parse_delimiter_hex() {
        assert_eq!(parse_delimiter("\\x00"), vec![0x00]);
        assert_eq!(parse_delimiter("\\x0a"), vec![0x0a]);
        assert_eq!(parse_delimiter("\\x41"), vec![0x41]);
        assert_eq!(parse_delimiter("\\xff"), vec![0xff]);
    }

    #[test]
    fn test_parse_delimiter_mixed() {
        assert_eq!(parse_delimiter("a\\nb"), vec![b'a', b'\n', b'b']);
        assert_eq!(parse_delimiter("\\x00,\\x01"), vec![0x00, b',', 0x01]);
    }

    #[test]
    fn test_parse_delimiter_invalid_hex() {
        assert_eq!(parse_delimiter("\\xgg"), vec![b'\\', b'x', b'g', b'g']);
        assert_eq!(parse_delimiter("\\x1"), vec![b'\\', b'x', b'1']);
    }

    #[test]
    fn test_parse_delimiter_escaped_backslash() {
        assert_eq!(parse_delimiter("\\\\"), vec![b'\\']);
        assert_eq!(parse_delimiter("\\a"), vec![b'a']);
    }

    #[test]
    fn test_find_delimiter_positions_empty_delimiter() {
        let data = b"hello world";
        let delimiter = b"";
        assert_eq!(find_delimiter_positions(data, delimiter), vec![0]);
    }

    #[test]
    fn test_find_delimiter_positions_single_char() {
        let data = b"a,b,c,d";
        let delimiter = b",";
        assert_eq!(find_delimiter_positions(data, delimiter), vec![0, 2, 4, 6]);
    }

    #[test]
    fn test_find_delimiter_positions_multi_char() {
        let data = b"foo::bar::baz";
        let delimiter = b"::";
        assert_eq!(find_delimiter_positions(data, delimiter), vec![0, 5, 10]);
    }

    #[test]
    fn test_find_delimiter_positions_no_match() {
        let data = b"hello world";
        let delimiter = b"xyz";
        assert_eq!(find_delimiter_positions(data, delimiter), vec![0]);
    }

    #[test]
    fn test_find_delimiter_positions_at_start() {
        let data = b",a,b,c";
        let delimiter = b",";
        assert_eq!(find_delimiter_positions(data, delimiter), vec![0, 1, 3, 5]);
    }

    #[test]
    fn test_find_delimiter_positions_at_end() {
        let data = b"a,b,c,";
        let delimiter = b",";
        assert_eq!(find_delimiter_positions(data, delimiter), vec![0, 2, 4, 6]);
    }

    #[test]
    fn test_find_delimiter_positions_consecutive() {
        let data = b"a,,b";
        let delimiter = b",";
        assert_eq!(find_delimiter_positions(data, delimiter), vec![0, 2, 3]);
    }

    #[test]
    fn test_find_delimiter_positions_empty_data() {
        let data = b"";
        let delimiter = b",";
        assert_eq!(find_delimiter_positions(data, delimiter), vec![0]);
    }

    #[test]
    fn test_find_delimiter_positions_delimiter_longer_than_data() {
        let data = b"ab";
        let delimiter = b"abc";
        assert_eq!(find_delimiter_positions(data, delimiter), vec![0]);
    }

    #[test]
    fn test_find_delimiter_positions_overlapping() {
        let data = b"aaa";
        let delimiter = b"aa";
        assert_eq!(find_delimiter_positions(data, delimiter), vec![0, 2]);
    }
}