use std::io::{self, Write};

use anyhow::Result;

/// Read a line of input from stdin with a prompt label.
pub fn prompt_input(label: &str) -> Result<String> {
    print!("{label}: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}
