fn main() {
    if let Err(error) = nsfy_desktop::cli::run() {
        eprintln!("nsfy-cli: {error}");
        std::process::exit(1);
    }
}
