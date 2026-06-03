#[cfg(windows)]
fn main() {
    winresource::WindowsResource::new()
        .set_icon("assets/app.ico")
        .compile()
        .expect("compile Windows resources");
}

#[cfg(not(windows))]
fn main() {}
