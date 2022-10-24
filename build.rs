fn main() {
    #[cfg(windows)]
    {
        // Embed application ICON
        embed_resource::compile("windows_icon.rc");
    }
}
