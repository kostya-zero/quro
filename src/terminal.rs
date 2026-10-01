use colored::Colorize;

pub fn print_error(msg: &str) {
    eprintln!("{}: {}", "error".bright_red().bold(), msg);
}

pub fn print_warn(msg: &str) {
    eprintln!("{}: {}", "warn".bright_yellow().bold(), msg);
}

pub fn escape_control_chars(value: String) -> String {
    if !value.chars().any(char::is_control) {
        return value;
    }

    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        if character.is_control() {
            escaped.extend(character.escape_default());
        } else {
            escaped.push(character);
        }
    }

    escaped
}
