use std::error::Error;

pub fn run() -> Result<(), Box<dyn Error>> {
    Ok(())
}

pub fn render_error(err: &dyn Error) {
    eprintln!("error: {err}");
}
