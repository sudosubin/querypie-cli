fn main() {
    std::process::exit(match querypie_cli::cli::run() {
        Ok(()) => 0,
        Err(err) => {
            querypie_cli::cli::render_error(&err);
            1
        }
    });
}
