fn main() {
    let version = env!("CARGO_PKG_VERSION");
    println!("{}", cadkernel::version_banner(version));
    cadkernel_viewer::run_gui();
}
