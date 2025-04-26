const GEOGRAPHICLIB_SRC: &str = "vendor/geographiclib/src";

fn geographiclib_cpp_files() -> Result<Vec<String>, String> {
    let mut files: Vec<String> = Vec::new();

    for entry in std::fs::read_dir(GEOGRAPHICLIB_SRC)
        .map_err(|_| "unable to read GeographicLib source directory".to_owned())?
    {
        let entry = entry.map_err(|_| "could not read directory entry")?;
        let file_name = entry.file_name().into_string().map_err(|os_string| {
            format!(
                "unable to convert file name {:?} to string",
                os_string.to_string_lossy()
            )
        })?;
        let file_type = entry
            .file_type()
            .map_err(|_| format!("unable to extract file type from {:?}", file_name))?;
        if file_type.is_file() && entry.path().extension().and_then(|s| s.to_str()) == Some("cpp") {
            files.push(GEOGRAPHICLIB_SRC.to_owned() + "/" + &file_name);
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
        .files(geographiclib_cpp_files().unwrap())
        .flag("-I../geo/include")
        .flag("-I../geo/vendor/geographiclib/include")
        .compile("geocxx");
}
