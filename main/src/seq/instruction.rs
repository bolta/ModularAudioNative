use super::{
	common::*,
};

use crate::core::{
	common::*,
};

#[derive(Clone, Debug)]
pub enum Instruction {
	// TODO tag は intern した文字列にする
	Note { tag: String, note_on: bool },
	Value { tag: String, value: Sample },
	Wait(i32),

	NewVar { name: String, value: i32 }, // TODO 型をつける
	DecrVar { name: String },
	DeleteVar { name: String },
	Call { seq_name: String },
	Jump { seq_name: Option<String>, pos: InstructionIndex },
	If0 { var: String, then: Box<Instruction> }, // いずれもっと汎用的な instrc で置換できるかもしれない
}
