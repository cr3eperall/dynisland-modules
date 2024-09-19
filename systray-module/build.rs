fn main() {
    // this will compile the ui files into a single gresource file, ready to be used by the module
    glib_build_tools::compile_resources(
        &["resources"],
        "resources/resources.gresource.xml",
        "compiled.gresource",
    )
}
