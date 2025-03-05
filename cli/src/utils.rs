use dialoguer::console::style;

/// Print a section header with consistent styling
pub fn print_section_header(title: &str) {
    println!("\n{}", style(format!("━━━ {} ━━━", title)).cyan().bold());
}

/// Print a success message with an emoji and optional details
pub fn print_success(message: &str, details: Option<&str>) {
    println!(
        "\n{}  {}{}",
        style("✓").green().bold(),
        style(message).green(),
        details.map_or(String::new(), |d| format!("\n   {}", style(d).dim()))
    );
}

/// Print an info message with consistent styling
pub fn print_info(message: &str) {
    println!("{}", style(format!("ℹ {}", message)).blue());
}
