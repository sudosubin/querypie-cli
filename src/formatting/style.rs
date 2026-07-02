pub(crate) fn success_icon() -> String {
    green("✓")
}

pub(crate) fn error_icon() -> String {
    red("X")
}

pub(crate) fn null_value() -> String {
    gray("NULL")
}

fn green(text: &str) -> String {
    color(text, anstyle::AnsiColor::Green)
}

fn red(text: &str) -> String {
    color(text, anstyle::AnsiColor::Red)
}

fn gray(text: &str) -> String {
    color(text, anstyle::AnsiColor::BrightBlack)
}

fn color(text: &str, color: anstyle::AnsiColor) -> String {
    let style = anstyle::Style::new().fg_color(Some(color.into()));
    format!("{style}{text}{style:#}")
}
