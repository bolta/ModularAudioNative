mod core;
mod mml;
mod node;

use crate::core::node::*;
use crate::core::node_handle::*;
use crate::core::node_host::*;
use crate::node::arith::*;
use crate::node::osc::*;
use crate::node::prim::*;
use combine::Parser;

fn main() {
	// let mut host = NodeHost::new(44100, 16, 1);
	// let freq = constant(&mut host, 440_f32);
	// // let osc =
	// // 		sin_osc(
	// // 				add(
	// // 						freq,
	// // 						mul(
	// // 								sin_osc(constant(&mut host, 1320_f32)),
	// // 								constant(&mut host, 660_f32))));

	// let osc =
	// 		pulse_osc(constant(&mut host, 440_f32), constant(&mut host, 0.25_f32));



	// 								let host_shared = std::rc::Rc::new(std::cell::RefCell::new(host));
	// NodeHost::play(host_shared);

//	println!("{}", crate::mml::default::default_mml_parser::parse_integer("-0123000"));
	// crate::mml::default::default_mml_parser::boke();
	let result = crate::mml::default::default_mml_parser::compilation_unit().parse("o4l8v15");
	println!("{}", "hoge");
	// println!("{}", result);
	println!("{:?}", result);
}

