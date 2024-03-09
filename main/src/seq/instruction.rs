use super::{
	common::*,
};

use crate::core::{
	common::*,
};

#[derive(Clone, Debug)]
pub enum Instruction {
	Nop,
	// TODO tag は intern した文字列にする
	Note { tag: String, note_on: bool },
	Value { tag: String, key: String, value: Sample },
	Wait(i32),

	NewVar { name: String, value: i32 }, // TODO 型をつける
	DecrVar { name: String },
	DeleteVar { name: String },
	Call { seq_name: String },
	JumpAbs { seq_name: Option<String>, pos: InstructionIndex },
	JumpRel { offset: i32 },
	If0 { var: String, then: Box<Instruction> }, // いずれもっと汎用的な instrc で置換できるかもしれない
	EnterSkipMode,
	ExitSkipMode,
}
