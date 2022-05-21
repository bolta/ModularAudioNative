use crate::core::common::Sample;
pub trait BinaryOp {
	fn oper(lhs: Sample, rhs: Sample) -> Sample;
}

 ////
//// arithmetic operations

pub struct AddOp { }
impl BinaryOp for AddOp { fn oper(lhs: Sample, rhs: Sample) -> Sample { lhs + rhs } }
pub struct SubOp { }
impl BinaryOp for SubOp { fn oper(lhs: Sample, rhs: Sample) -> Sample { lhs - rhs } }
pub struct MulOp { }
impl BinaryOp for MulOp { fn oper(lhs: Sample, rhs: Sample) -> Sample { lhs * rhs } }
pub struct DivOp { }
impl BinaryOp for DivOp { fn oper(lhs: Sample, rhs: Sample) -> Sample { lhs / rhs } }
pub struct RemOp { }
impl BinaryOp for RemOp { fn oper(lhs: Sample, rhs: Sample) -> Sample { lhs % rhs } }
pub struct PowOp { }
impl BinaryOp for PowOp { fn oper(lhs: Sample, rhs: Sample) -> Sample { lhs.powf(rhs) } }

 ////
//// comparison operations

pub struct LtOp { }
impl BinaryOp for LtOp { fn oper(lhs: Sample, rhs: Sample) -> Sample { bool_to_sample(lhs < rhs) } }
pub struct LeOp { }
impl BinaryOp for LeOp { fn oper(lhs: Sample, rhs: Sample) -> Sample { bool_to_sample(lhs <= rhs) } }
pub struct EqOp { }
impl BinaryOp for EqOp { fn oper(lhs: Sample, rhs: Sample) -> Sample { bool_to_sample(lhs == rhs) } }
pub struct NeOp { }
impl BinaryOp for NeOp { fn oper(lhs: Sample, rhs: Sample) -> Sample { bool_to_sample(lhs != rhs) } }
pub struct GtOp { }
impl BinaryOp for GtOp { fn oper(lhs: Sample, rhs: Sample) -> Sample { bool_to_sample(lhs > rhs) } }
pub struct GeOp { }
impl BinaryOp for GeOp { fn oper(lhs: Sample, rhs: Sample) -> Sample { bool_to_sample(lhs >= rhs) } }

 ////
//// logical operations

pub struct AndOp { }
impl BinaryOp for AndOp { fn oper(lhs: Sample, rhs: Sample) -> Sample { bool_binary(lhs, rhs, |lhs, rhs| lhs && rhs) } }
pub struct OrOp { }
impl BinaryOp for OrOp { fn oper(lhs: Sample, rhs: Sample) -> Sample { bool_binary(lhs, rhs, |lhs, rhs| lhs || rhs) } }

// TODO 定数を ModDL と共通化
fn bool_to_sample(b: bool) -> Sample { if b { 1f32 } else { -1f32 } }
fn sample_to_bool(s: Sample) -> bool { s > 0f32 }
fn bool_binary(lhs: Sample, rhs: Sample, op: fn (lhs: bool, rhs: bool) -> bool) -> Sample {
	bool_to_sample(op(sample_to_bool(lhs), sample_to_bool(rhs)))
}



// TODO 仮置き
pub trait Calc {
	fn arg_count() -> i32;
	fn calc(args: &Vec<Sample>) -> Sample;
}

pub struct LogCalc { }
impl Calc for LogCalc {
	fn arg_count() -> i32 { 1 }
	fn calc(args: &Vec<Sample>) -> Sample { args[0].ln() }
}
