fn main() {
    glib_build_tools::compile_resources(
        &["data/resources"],
        "data/resources/gresources.xml",
        "gresources.gresource",
    );
}
