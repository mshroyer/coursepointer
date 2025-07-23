const GEOGRAPHICLIB_SRC: &str = "geographiclib/src";

#[cfg(feature = "full-geolib")]
mod sources {
    use std::path::{Path, PathBuf};

    use crate::GEOGRAPHICLIB_SRC;

    fn list_cpp_files<P: AsRef<Path>>(dir: P) -> Result<Vec<PathBuf>, String> {
        let mut files: Vec<PathBuf> = Vec::new();

        for entry in std::fs::read_dir(dir)
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
                .map_err(|_| format!("unable to determine entry type from {}", file_name))?;
            let path = entry.path();
            if file_type.is_file() && path.extension().and_then(|s| s.to_str()) == Some("cpp") {
                files.push(path)
            }
        }
        Ok(files)
    }

    pub fn geographiclib_cpp() -> Result<Vec<PathBuf>, String> {
        list_cpp_files(GEOGRAPHICLIB_SRC)
    }
}

#[cfg(not(feature = "full-geolib"))]
mod sources {
    use std::path::PathBuf;

    use crate::GEOGRAPHICLIB_SRC;

    pub fn geographiclib_cpp() -> Result<Vec<PathBuf>, String> {
        // A minimal subset of source files needed for our current FFI.
        let filenames = vec![
            "DST.cpp",
            "EllipticFunction.cpp",
            "Geocentric.cpp",
            "Geodesic.cpp",
            "GeodesicExact.cpp",
            "GeodesicLine.cpp",
            "GeodesicLineExact.cpp",
            "Gnomonic.cpp",
            "Math.cpp",
        ];

        Ok(filenames
            .into_iter()
            .map(|f| {
                let mut p: PathBuf = GEOGRAPHICLIB_SRC.into();
                p.push(f);
                p
            })
            .collect::<Vec<PathBuf>>())
    }
}

fn main() {
    if !std::fs::exists(GEOGRAPHICLIB_SRC).unwrap() {
        panic!(concat!(
            "geographiclib/src is missing. ",
            "Did you run git submodule init && git submodule update first?"
        ))
    }

    let ver = rustc_version::version().expect("Failed to get rustc version");
    println!("cargo:rustc-env=RUSTC_VERSION={ver}");

    let target = std::env::var("TARGET").expect("TARGET not set");

    if target != "wasm32-unknown-unknown" {
        // Thankfully GeographicLib has a pretty simple build, so we can just compile
        // all the source files here rather than go through CMake.
        cc::Build::new()
            .cpp(true)
            .flag_if_supported("-std=c++11")
            .flag_if_supported("/std:c++11")
            .file("src/shim.cpp")
            .files(sources::geographiclib_cpp().unwrap())
            .flag("-I./include")
            .flag("-I./geographiclib/include")
            .compile("geocxx");

        for file in sources::geographiclib_cpp().unwrap() {
            println!("cargo:rerun-if-changed={}", file.display());
        }
        println!("cargo:rerun-if-changed=src/shim.cpp");
    }

    #[cfg(target_os = "windows")]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("res/coursepointer.ico");
        res.set_language(winapi::um::winnt::MAKELANGID(
            winapi::um::winnt::LANG_ENGLISH,
            winapi::um::winnt::SUBLANG_ENGLISH_US,
        ));
        res.compile().unwrap();
    }
}
