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
mod vis;
mod wave;

use crate::moddl::{
	player,
	player_option::*,
};

use std::{
	env, process::exit, thread,
};

// パーザを切り出したがエラーを参照するため必要
extern crate nom;

fn main() {
	match env::args().nth(1) {
		None => {
			eprintln!("Please specify a moddl file path.");
			exit(1);
		}
		Some(moddl_path) => {
			if let Err(e) = player::play(&PlayerOptions {
				moddl_path,
				// output: PlayerOutput::Wav { path: "out.wav".to_string() },
				output: PlayerOutput::Audio,
			}) {
				println!("error: {}: {}", e.loc, e.body);
				exit(1);
			}
		}
	}
}
