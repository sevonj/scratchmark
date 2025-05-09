fn main() {
    glib_build_tools::compile_resources(
        &["resources"],
        "resources/gresources.xml",
        "gresources.gresource",
    );
}
