use std::process::Command;

fn main() {
    glib_build_tools::compile_resources(
        &["data/resources"],
        "data/resources/gresources.xml",
        "gresources.gresource",
    );

    let output = Command::new("glib-compile-schemas")
        .arg("data")
        .output()
        .expect("failed to compile settings schema");
    println!("{:?}", output.stdout);
}
