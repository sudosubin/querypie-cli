use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

pub(crate) fn print_table<I>(headers: &[&str], rows: I, truncate: bool)
where
    I: IntoIterator<Item = Vec<String>>,
{
    let rows = rows.into_iter().collect::<Vec<_>>();
    if headers.is_empty() {
        for row in rows {
            println!("{}", row.join("\t"));
        }
        return;
    }

    let mut widths = headers
        .iter()
        .map(|header| display_width(header))
        .collect::<Vec<_>>();
    for row in &rows {
        for (index, cell) in row.iter().enumerate() {
            if let Some(width) = widths.get_mut(index) {
                *width = (*width).max(display_width(cell));
            }
        }
    }
    if truncate {
        fit_widths_to_terminal(&mut widths);
    }

    print_row(
        headers.iter().map(|value| value.to_string()).collect(),
        &widths,
    );
    print_row(
        widths.iter().map(|width| "-".repeat(*width)).collect(),
        &widths,
    );
    for row in rows {
        print_row(row, &widths);
    }
}

fn print_row(row: Vec<String>, widths: &[usize]) {
    let cells = row
        .into_iter()
        .enumerate()
        .map(|(index, value)| {
            let width = widths.get(index).copied().unwrap_or(0);
            pad_to_width(truncate_to_width(&value, width), width)
        })
        .collect::<Vec<_>>();
    println!("{}", cells.join("  "));
}

fn fit_widths_to_terminal(widths: &mut [usize]) {
    let Some(term_width) = terminal_width() else {
        return;
    };
    let separators = widths.len().saturating_sub(1) * 2;
    let min_width = 8;
    while widths.iter().sum::<usize>() + separators > term_width {
        let Some(width) = widths.iter_mut().filter(|width| **width > min_width).max() else {
            break;
        };
        *width -= 1;
    }
}

fn terminal_width() -> Option<usize> {
    terminal_size::terminal_size_of(std::io::stdout())
        .map(|(terminal_size::Width(width), _)| width as usize)
}

fn truncate_to_width(value: &str, width: usize) -> String {
    if display_width(value) <= width {
        return value.to_string();
    }
    if width == 0 {
        return String::new();
    }
    if width == 1 {
        return "…".to_string();
    }

    let ellipsis = "…";
    let target = width.saturating_sub(display_width(ellipsis));
    let mut out = String::new();
    let mut used = 0;
    for ch in value.chars() {
        let ch_width = ch.width().unwrap_or(0);
        if used + ch_width > target {
            break;
        }
        out.push(ch);
        used += ch_width;
    }
    out.push_str(ellipsis);
    out
}

fn pad_to_width(value: String, width: usize) -> String {
    let padding = width.saturating_sub(display_width(&value));
    format!("{value}{}", " ".repeat(padding))
}

fn display_width(value: &str) -> usize {
    UnicodeWidthStr::width(strip_ansi(value).as_str())
}

fn strip_ansi(value: &str) -> String {
    let mut out = String::new();
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' && chars.peek() == Some(&'[') {
            let _ = chars.next();
            for ch in chars.by_ref() {
                if ch.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(ch);
        }
    }
    out
}
