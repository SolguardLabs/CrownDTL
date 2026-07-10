fn main() {
    if let Err(err) = crown_dtl::runtime::run_from_env() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}
