use std::{
	path::{
		Path,
		PathBuf,
	},
};

pub fn resolve_path(moddl_path: &str, base_moddl_path: &str) -> PathBuf {
	let moddl_path = Path::new(moddl_path);
	if moddl_path.is_absolute() {
		moddl_path.to_path_buf()
	} else {
		// 相対パスは必ず ./ か ../ から始まるものとする。
		// いきなり名前を書くのは、標準ライブラリ等の、置き場所が別途定義されたライブラリの指定方法として予約しておく
		if moddl_path.starts_with("./") || moddl_path.starts_with("../")
				|| moddl_path.starts_with(r".\") || moddl_path.starts_with(r"..\") {
			let joined = Path::new(base_moddl_path).parent().unwrap().join(moddl_path);
			// ./../ などの冗長な表現は残るが、ここではそのままにする。
			// これを解決するには canonicalize する必要があるが、ファイルパスが実在しないとエラーになるので
			// テスト等で都合が悪い
			joined
			// Ok(Path::new(base_moddl_path).parent().unwrap().join(moddl_path))//.canonicalize().map_err(Error::File)
		} else {
			unimplemented!("relative path must start with ./ or ../ so far")
		}
	}
}

#[cfg(test)]
#[test]
fn test_resolve_path() {
	fn test(moddl_path: &str, base_moddl_path: &str, expected: &str) {
		assert_eq!(resolve_path(moddl_path, base_moddl_path), PathBuf::from(expected));
	}
	test(r"./sub.moddl", r"main.moddl", r"./sub.moddl");
	test(r"./sub.moddl", r"./main.moddl", r"./sub.moddl");
	test(r"../sub.moddl", r"./main.moddl", r"./../sub.moddl");
	test(r"../sub.moddl", r"/main.moddl", r"/../sub.moddl"); // 変なパスだがここでは特に考慮しない
	test(r"../lib/sub.moddl", r"./song/main.moddl", r"./song/../lib/sub.moddl");
	#[cfg(not(target_os = "windows"))] // TODO Unix 系環境でもテストする
	{
		test(r"/sub.moddl", r"/foo/bar/main.moddl", r"/sub.moddl");
	}
	#[cfg(target_os = "windows")]
	{
		test(r"../sub.moddl", r"C:\foo\bar\main.moddl", r"C:\foo\bar\..\sub.moddl");
		test(r"C:\sub.moddl", r"C:\foo\bar\main.moddl", r"C:\sub.moddl");
	}
}

#[cfg(test)]
#[test]
#[should_panic(expected = "relative path must start with ./ or ../ so far")]
fn test_resolve_path_panic() {
	resolve_path(r"sub.moddl", r"main.moddl");
}
