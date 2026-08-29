fn main() {
	// main プロジェクト（moddl）のバイナリターゲット名を取得
	let metadata = cargo_metadata::MetadataCommand::new()
			.exec()
			.expect("Failed to get cargo metadata");

	let target = metadata.packages
			.iter()
			.filter(|package| package.name == "moddl")
			.flat_map(|package| package.targets.iter())
			.find(|target| target.kind.iter().any(|k| k == "bin"))
			.expect("Failed to find moddl binary target");

	println!("cargo:rustc-env=MODDL_BIN_NAME={}", target.name);
}
