# pdf2fountain

A fast, standalone Rust library and command-line tool to convert screenplay PDFs into clean, standard [Fountain](https://fountain.io) screenplays.

## Features

- **Adaptive Layout Detection**: Works across US Letter and A4 page sizes without rigid hardcoded margins.
- **Accurate Screenplay Elements**: Detects Scene Headings, Character names, Dialogue, Parentheticals, Transitions, and Title Pages.
- **Standalone & Fast**: Pure Rust with minimal dependencies.

## Usage

```bash
# Convert a screenplay PDF to Fountain
pdf2fountain script.pdf -o script.fountain

# Convert specific pages
pdf2fountain script.pdf --pages 1-10 -o script.fountain

# Output directly to stdout
pdf2fountain script.pdf > script.fountain
```

## License

MIT
