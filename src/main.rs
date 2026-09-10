use std::env;
use std::fs;
use std::process;

fn print_help() {
    eprintln!(
        r#"pdf2fountain - Screenplay PDF to Fountain converter

USAGE:
    pdf2fountain <input.pdf> [OPTIONS]

OPTIONS:
    -o, --output <file>       Write Fountain output to the specified file path
    -p, --pages <range>       Convert specific pages (e.g. 1-10, 5, or 2-)
    -h, --help                Show this help message
    -v, --version             Show version

EXAMPLES:
    pdf2fountain screenplay.pdf -o screenplay.fountain
    pdf2fountain screenplay.pdf --pages 1-5 -o sample.fountain
    pdf2fountain screenplay.pdf > screenplay.fountain
"#
    );
}

fn parse_page_range(s: &str) -> Option<(usize, usize)> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() == 1 {
        let n: usize = parts[0].trim().parse().ok()?;
        Some((n, n))
    } else if parts.len() == 2 {
        let start: usize = if parts[0].trim().is_empty() { 1 } else { parts[0].trim().parse().ok()? };
        let end: usize = if parts[1].trim().is_empty() { usize::MAX } else { parts[1].trim().parse().ok()? };
        Some((start, end))
    } else {
        None
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_help();
        process::exit(1);
    }

    if args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
        print_help();
        process::exit(0);
    }

    if args.contains(&"--version".to_string()) || args.contains(&"-v".to_string()) {
        println!("pdf2fountain {}", env!("CARGO_PKG_VERSION"));
        process::exit(0);
    }

    let input_path = &args[1];
    let mut output_path = None;
    let mut page_range = None;

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => {
                if i + 1 < args.len() {
                    output_path = Some(args[i + 1].clone());
                    i += 2;
                    continue;
                }
            }
            "-p" | "--pages" => {
                if i + 1 < args.len() {
                    page_range = parse_page_range(&args[i + 1]);
                    i += 2;
                    continue;
                }
            }
            _ => {}
        }
        i += 1;
    }

    let parse_result = if let Some(range) = page_range {
        pdf2fountain::parse_pdf_file_to_fountain_pages(input_path, range)
    } else {
        pdf2fountain::parse_pdf_file_to_fountain(input_path)
    };

    match parse_result {
        Ok(fountain) => {
            if let Some(out_file) = output_path {
                if let Err(e) = fs::write(&out_file, &fountain) {
                    eprintln!("error writing output file: {e}");
                    process::exit(1);
                }
            } else {
                print!("{fountain}");
            }
        }
        Err(e) => {
            eprintln!("error parsing pdf: {e}");
            process::exit(1);
        }
    }
}
