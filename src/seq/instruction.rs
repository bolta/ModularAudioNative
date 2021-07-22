use crate::core::{
	common::*,
};

#[derive(Clone)]
pub enum Instruction {
	// TODO tag は intern した文字列にする
	Note { tag: String, note_on: bool },
	Value { tag: String, value: Sample },
	Wait { ticks: i32 },
}
