use static_files::resource_dir;

// Generates module for serving files in static directory
fn main() -> std::io::Result<()> {
    resource_dir("./static").build()
}
