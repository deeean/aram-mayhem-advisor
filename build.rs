fn main() {
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("data/icon.ico");
        res.compile().expect("Failed to compile resources");
    }
}
