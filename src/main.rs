fn main() {
    std::process::exit(match querypie::cli::run() {
        Ok(()) => 0,
        Err(err) => {
            querypie::cli::render_error(err.as_ref());
            1
        }
    });
}
