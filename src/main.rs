#![allow(dead_code)]
#![type_length_limit="106079600"]

mod core;
mod mml;
mod moddl;
mod node;
mod seq;
mod wave;

use crate::moddl::{
	player,
};

use std::{
	env,
};

extern crate nom;

fn main() {
	match env::args().nth(1) {
		None => {
			eprintln!("Please specify the moddl file path.");
		}
		Some(moddl_path) => {
			if let Err(e) = player::play_file(moddl_path.as_str()) {
				eprintln!("An error occurred: {:?}", e);
			}
		}
	}
}
