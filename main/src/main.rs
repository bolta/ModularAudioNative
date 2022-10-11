#![allow(dead_code)]
#![type_length_limit="300000000"]

// マクロを提供するモジュール（common::parser）はマクロを使うモジュールより先に、
// かつ #[macro_use] をつけて宣言する必要がある
// https://stackoverflow.com/questions/26731243/how-do-i-use-a-macro-across-module-files
#[macro_use]
mod common;

mod calc;
mod core;
mod mml;
mod moddl;
mod node;
mod seq;
mod wave;

use crate::moddl::{
	player,
	player_output::*,
};

use std::{
	env,
	process::{
		exit,
	},
};

// パーザを切り出したがエラーを参照するため必要
extern crate nom;

fn main() {
	match env::args().nth(1) {
		None => {
			eprintln!("Please specify the moddl file path.");
			exit(1);
		}
		Some(moddl_path) => {
			if let Err(e) = player::play(moddl_path.as_str(), PlayerOutput::Audio) {
				eprintln!("An error occurred: {:?}", e);
				exit(1);
			}
		}
	}
}
