use proc_macro::TokenStream;
// use proc_macro2::to_string();
// extern crate syn;
use quote::quote;
use syn::{
	ImplItem,
	ImplItemMethod,
	Item,
	ItemImpl,
	parse_macro_input,
};
use proc_macro2::{
	Ident,
	Span,
};

// TODO 実装を node_macro_impl に移し、ここではそれを呼び出すだけにする
// TODO そして node_macro_impl にテストを書く
#[proc_macro_attribute]
pub fn node_impl(_attr: TokenStream, item: TokenStream) -> TokenStream {
	// let ret = item.clone();
	let mut ast = parse_macro_input!(item as ItemImpl);
	let method_names: Vec<String> = ast.items.iter().filter_map(|item| match item {
		ImplItem::Method(meth) => Some(meth.sig.ident.to_string()),
		_ => None,
	}).collect();

	// println!("{:?}", &ast.self_ty);

	macro_rules! add_implementation_marker {
		($target_name: expr) => {
			let marker_name = format!("implements_{}", $target_name);
			let marker_name_ident = Ident::new(marker_name.as_str(), Span::call_site());
			if ! method_names.contains(& marker_name) {
				// println!("{:?}", &ast.self_ty);
				let meth: TokenStream = if method_names.contains(& $target_name.to_string()) {
					// println!("adding method {}: true", marker_name);
					quote! {
						fn #marker_name_ident(&self) -> bool { true }
					}
				} else {
					// println!("adding method {}: false", marker_name);
					quote! {
						fn #marker_name_ident(&self) -> bool { false }
					}
				}.into();
				let meth_ast = parse_macro_input!(meth as ImplItemMethod);
				ast.items.push(ImplItem::Method(meth_ast));
			}
		}
	}
	add_implementation_marker!("execute");
	add_implementation_marker!("update");

	let meth: TokenStream = quote! {
		fn base(&self) -> &NodeBase { &self.base_ }
	}.into();
	let meth_ast = parse_macro_input!(meth as ImplItemMethod);
	ast.items.push(ImplItem::Method(meth_ast));

	// 関数にするとなぜかコンパイルエラーになるためマクロで共通化
	// add_implementation_marker(&mut ast, &method_names, "execute");
	use quote::ToTokens;
	Item::Impl(ast).into_token_stream().into()
}

// fn add_implementation_marker(ast: &mut ItemImpl, existing_method_names: &Vec<String>, target_method_name: &str) {
// 	let marker_name = format!("implements_{}", target_method_name);
// 	let marker_name_ident = Ident::new(marker_name.as_str(), Span::call_site());
// 	if existing_method_names.contains(&marker_name) { return; }

// 	let meth: TokenStream = if existing_method_names.contains(& target_method_name.to_string()) {
// 		println!("adding method {} (true)", &marker_name);
// 		quote! {
// 			fn #marker_name_ident(&self) -> bool { true }
// 		}
// 	} else {
// 		println!("adding method {} (false)", &marker_name);
// 		quote! {
// 			fn #marker_name_ident(&self) -> bool { false }
// 		}
// 	}.into();
//  // expected `()`, found struct `proc_macro::TokenStream`
// 	let meth_ast = parse_macro_input!(meth as ImplItemMethod);
// 	ast.items.push(ImplItem::Method(meth_ast));
// }
