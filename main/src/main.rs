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
	path::PathBuf, process::exit, thread
};

// パーザを切り出したがエラーを参照するため必要
extern crate nom;


use clap::{ error::ErrorKind, ArgGroup, CommandFactory, Error, Parser, ValueEnum };
use regex::Regex;

#[derive(Debug, Clone, ValueEnum)]
enum CliOutput { Audio, Stdout, Null }

#[derive(Debug, Parser)]
#[command(group(ArgGroup::new("output_spec").required(false).args(["output", "output_file"])))]
struct CliArgs {
	#[arg(help = "path to moddl file to play")]
	moddl_path: PathBuf,

	#[arg(long, help = "dumps AST of source file in JSON")]
	dump_ast: bool,

	#[arg(long, help = "omits building machines and playing song")]
	no_play: bool,

	#[arg(long, short('O'), help = "specifies output type")]
	output: Option<CliOutput>,

	#[arg(long, short('o'), help = "specifies output file path")]
	output_file: Option<PathBuf>,

	#[arg(long, short('S'), help = "specifies stack size for moddl processor")]
	stack_size: Option<String>,
}

const DEFAULT_STACK_SIZE: usize = 10 * 1024 * 1024;

fn parse_stack_size(stack_size_arg: &Option<String>) -> Result<usize, Error> {
	match stack_size_arg {
		None => Ok(DEFAULT_STACK_SIZE),
		Some(size) => {
			let patt = Regex::new(r"^(\d+)([kKmM]?)$").unwrap();
			let caps = patt.captures(size.as_str());
			match &caps {
				None => Err(CliArgs::command().error(ErrorKind::InvalidValue,
						"stack size must be positive integer + optional (k|K|m|M)")),
				Some(caps) => {
					let num = caps.get(1).unwrap().as_str().parse::<usize>().unwrap();
					let unit = match caps.get(2).unwrap().as_str() {
						"" => 1,
						"k" | "K" => 1024,
						"m" | "M" => 1024 * 1024,
						_ => unreachable!(),
					};
					Ok(num * unit)
				},
			}
		}
	}
}

fn main() {
	let opts = CliArgs::parse();
	let stack_size = parse_stack_size(&opts.stack_size).unwrap_or_else(|e| e.exit());

	let player_opts = PlayerOptions {
		moddl_path: opts.moddl_path,
		dump_ast: opts.dump_ast,
		no_play: opts.no_play,
		output: match opts.output_file {
			Some(path) => PlayerOutput::Wav { path },
			None => match opts.output {
				None
				| Some(CliOutput::Audio) => PlayerOutput::Audio,
				Some(CliOutput::Stdout) => PlayerOutput::Stdout,
				Some(CliOutput::Null) => PlayerOutput::Null,
			}
		}
	};

	// Spawn thread with explicit stack size
	let play_thread = thread::Builder::new()
			.stack_size(stack_size)
			// .spawn(main_)
			.spawn(move || {
				if let Err(e) = player::play(&player_opts) {
					println!("error: {}: {}", e.loc, e.body);
					exit(1);
				}
			})
			.unwrap();

	// Wait for thread to join
	play_thread.join().unwrap();
}
