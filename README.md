# pdf2fountain

`pdf2fountain` is a lightweight, pure Rust library and command-line tool that parses screenplay PDFs and converts them into standard [Fountain](https://fountain.io) markdown.

It was designed specifically to convert industry-standard formatted screenplay PDFs into editable plain text without using OCR, external Python wrappers, or heavy runtime dependencies.

Part of [iyal.ink](https://iyal.ink) — the Open Source Revolution for Film Writers.

---

## Installation

### CLI Tool
```bash
cargo install pdf2fountain
```

### Library Dependency
Add to your `Cargo.toml`:
```toml
[dependencies]
pdf2fountain = "0.1.1"
```
Or via cargo:
```bash
cargo add pdf2fountain
```

---


## What the Crate Does

Screenplay PDFs flatten semantic screenplay elements (scenes, character names, parentheticals, dialogue, transitions) into positioned text strings. `pdf2fountain` reconstructs the screenplay structure through:

1. **Geometry Extraction (`geom.rs`)**:
   Reads PDF content streams, tracks coordinate transformation matrices (`cm`, `q`, `Q`), font dictionaries, and text placement operators (`Tm`, `Tj`, `TJ`), normalizing all text items to standard page coordinates.

2. **Adaptive Dynamic Layout Calibration (`calibrate.rs`)**:
   Unlike standard converters that assume fixed margin pixel offsets, `pdf2fountain` collects horizontal alignment histograms across the document to discover the script's exact column margins. Because of this, it handles both **A4** and **US Letter** screenplay layouts with equal precision.

3. **Screenplay Parser (`fsm.rs`)**:
   Processes text flow line by line:
   - **Scene Headings**: Formatted with leading `.` (e.g. `.INT. COFFEE SHOP - DAY #1#`). Detects and extracts scene numbers from left margins, right margins, or inline text.
   - **Action**: Formatted with leading `!` (e.g. `!The door swings open.`). Wraps multi-line sentences into single paragraphs while keeping distinct action blocks separated.
   - **Characters**: Formatted with leading `@` (e.g. `@SARAH`). Automatically strips print tags like `(CONT'D)`.
   - **Dialogue**: Merged into single lines attached directly beneath characters and parentheticals without unwanted line breaks.
   - **Parentheticals**: Consolidated into clean `(...)` blocks.
   - **Page Breaks**: Filters out running header page numbers and strips `(MORE)` markers, automatically reconnecting dialogues split across pages.
   - **Transitions**: Prefixed with `>` (e.g. `> CUT TO:`).
   - **Title Page**: Identifies title, author, and credit lines from cover pages.

4. **Fountain Formatter (`fountain.rs`)**:
   Renders the parsed screenplay elements into standard Fountain markup according to the Fountain syntax specifications.

---

## CLI Commands and Options

### Basic Syntax
```bash
pdf2fountain <input.pdf> [OPTIONS]
```

### Options

| Flag | Long Flag | Description | Example |
|---|---|---|---|
| `-o` | `--output <file>` | Write Fountain output to a file instead of stdout | `pdf2fountain script.pdf -o script.fountain` |
| `-p` | `--pages <range>` | Convert a specific page range or single page | `pdf2fountain script.pdf --pages 1-10 -o script.fountain` |
| `-h` | `--help` | Display CLI help and usage guide | `pdf2fountain --help` |
| `-v` | `--version` | Display current crate version | `pdf2fountain --version` |

### CLI Examples

**1. Convert an entire screenplay to a file:**
```bash
pdf2fountain screenplay.pdf -o screenplay.fountain
```

**2. Print Fountain output directly to stdout / pipe to another tool:**
```bash
pdf2fountain screenplay.pdf > screenplay.fountain
pdf2fountain screenplay.pdf | grep "^@"
```

**3. Convert a specific range of pages:**
```bash
# Convert pages 1 to 5
pdf2fountain screenplay.pdf --pages 1-5 -o sample.fountain

# Convert starting from page 2 to the end
pdf2fountain screenplay.pdf -p 2- -o remaining.fountain

# Convert only page 3
pdf2fountain screenplay.pdf -p 3 -o page3.fountain
```

---

## Rust Library API

### Parsing from a File Path

```rust
use pdf2fountain::parse_pdf_file_to_fountain;

fn main() -> Result<(), String> {
    let fountain_text = parse_pdf_file_to_fountain("my_script.pdf")?;
    println!("{fountain_text}");
    Ok(())
}
```

### Parsing Specific Page Ranges

```rust
use pdf2fountain::parse_pdf_file_to_fountain_pages;

fn main() -> Result<(), String> {
    // Parse pages 1 through 10
    let fountain_text = parse_pdf_file_to_fountain_pages("my_script.pdf", (1, 10))?;
    println!("{fountain_text}");
    Ok(())
}
```

### Parsing from In-Memory Bytes

```rust
use pdf2fountain::parse_pdf_to_fountain;

fn main() -> Result<(), String> {
    let pdf_bytes = std::fs::read("my_script.pdf").map_err(|e| e.to_string())?;
    let fountain_text = parse_pdf_to_fountain(&pdf_bytes)?;
    println!("{fountain_text}");
    Ok(())
}
```

---

## Running Tests

To run the integration and unit tests:

```bash
cargo test
```

## Other Projects

`pdf2fountain` is part of the [iyal.ink](https://iyal.ink) open-source ecosystem — building modern, privacy-focused, cross-platform tools for screenwriters and film storytellers:

| Project | Description |
|---|---|
| **[ActOne Screenplay](https://github.com/BeetleBot/ActOne-Screenplay)** | Elegant, cross-platform desktop screenplay editor designed for focused writing and structural outlining. |
| **[FountTUI](https://github.com/BeetleBot/FountTUI)** | Fast terminal-based UI (TUI) editor and reader for Fountain scripts. |

---

## License

MIT

