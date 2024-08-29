use std::env;
use std::process::Command;

fn main() {
	println!("cargo:rustc-link-search=native=lib");

	// Re-runs script if any files in res are changed  
	println!("cargo:rerun-if-changed=res/*");

	// println!("PROFILE: {}", &env::var("PROFILE").unwrap());
	// println!("OUT_DIR: {}", &env::var("OUT_DIR").unwrap());
	// println!("TARGET: {}", &env::var("TARGET").unwrap());
	let profile = &env::var("PROFILE").unwrap();
	let dest_dir = format!("..\\target\\{}", profile);
	println!("{dest_dir}");
	let _output = if cfg!(target_os = "windows") {
		Command::new("xcopy")
				.args(["/E", "/Y", "res", &dest_dir])
				.output()
				.expect("failed to execute process")
	} else {
		// TODO Windows 以外にも対応
		panic!("unsupported os")
	};
}
