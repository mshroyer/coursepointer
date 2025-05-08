use std::path::Path;
use std::path::PathBuf;

fn list_cpp_files<P: AsRef<Path>>(dir: P) -> Result<Vec<PathBuf>, String> {
    let mut files: Vec<PathBuf> = Vec::new();

    for entry in std::fs::read_dir(dir).map_err(|_| {
        "unable to read GeographicLib source directory; try git submodule init/update?".to_owned()
    })? {
        let entry = entry.map_err(|_| "could not read directory entry")?;
        let file_name = entry.file_name().into_string().map_err(|os_string| {
            format!(
                "unable to convert file name {:?} to string",
                os_string.to_string_lossy()
            )
        })?;
        let file_type = entry
            .file_type()
            .map_err(|_| format!("unable to determine entry type from {}", file_name))?;
        let path = entry.path();
        if file_type.is_file() && path.extension().and_then(|s| s.to_str()) == Some("cpp") {
            files.push(path)
        }
    }
    Ok(files)
}

fn main() {
    // Thankfully GeographicLib has a pretty simple build, so we can just compile all the source
    // files here rather than go through CMake.
    //
    // We could in principle identify the smallest set of .cpp files needed to build the functions
    // we need, which would slightly speed up the build, but that's a pain and the linker will
    // strip out what we don't need anyway.
    cxx_build::bridge("src/lib.rs")
        .file("src/shim.cc")
        .files(list_cpp_files("geographiclib/src").unwrap())
        .flag("-I../geographic/include")
        .flag("-I../geographic/geographiclib/include")
        .compile("geocxx");
}
