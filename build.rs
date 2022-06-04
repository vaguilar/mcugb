fn main() {
    pkg_config::Config::new().probe("sdl2").unwrap();
}
