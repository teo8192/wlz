use std::env;
use std::path::PathBuf;
use std::process::Command;

struct PkgConfig {
    libs: Vec<String>,
}

impl PkgConfig {
    pub fn new() -> Self {
        Self { libs: Vec::new() }
    }

    pub fn add_lib(mut self, lib: impl Into<String>) -> Self {
        self.libs.push(lib.into());
        self
    }

    fn pkg_config_wrapper<'a>(
        &self,
        arg: impl Into<Vec<&'a str>>,
        callback: fn(String) -> String,
    ) -> Vec<String> {
        let mut args = arg.into();
        args.extend(self.libs.iter().map(String::as_str));

        let output = Command::new("pkg-config")
            .args(&args)
            .output()
            .expect("unable to execute pkg-config");

        String::from_utf8(output.stdout)
            .expect("unable to parse output from pkg-config")
            .split([' ', '\n'])
            .map(String::from)
            .filter(|s| !s.is_empty())
            .map(callback)
            .collect()
    }

    pub fn libs(&self) -> Vec<String> {
        self.pkg_config_wrapper(["--libs"], |flag| {
            if let Some(c_lib) = flag.strip_prefix("-l") {
                format!("rustc-link-lib={c_lib}")
            } else {
                unimplemented!("only implemented -l flags, not '{flag}'")
            }
        })
    }

    pub fn cflags(&self) -> Vec<String> {
        self.pkg_config_wrapper(["--cflags"], |flag| {
            if let Some(lib_path) = flag.strip_prefix("-I") {
                format!("rustc-link-search={lib_path}")
            } else {
                unimplemented!("only implemented -I flags, not '{flag}'")
            }
        })
    }

    pub fn args(&self) -> Vec<String> {
        self.pkg_config_wrapper(["--cflags", "--libs"], |x| x)
    }
}

fn main() {
    // Tell cargo to look for shared libraries in the specified directory
    //println!("cargo:rustc-link-search=/path/to/lib");

    let pkg_config = PkgConfig::new()
        .add_lib("wayland-server")
        .add_lib("wlroots-0.19")
        .add_lib("xkbcommon");

    for cflag in pkg_config.cflags() {
        println!("cargo:{cflag}");
    }

    for lib_opt in pkg_config.libs() {
        println!("cargo:{lib_opt}");
    }

    let mut clang_args = pkg_config.args();
    clang_args.push(String::from("-DWLR_USE_UNSTABLE"));

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header("src/wrapper.h")
        .clang_args(clang_args)
        .blocklist_file(".*/math.h")
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
