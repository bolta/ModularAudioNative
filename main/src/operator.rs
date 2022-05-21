use crate::core::common::Sample;
pub trait BinaryOp {
	fn oper(lhs: Sample, rhs: Sample) -> Sample;
}

pub trait Calc {
	fn arg_count() -> i32;
	fn calc(args: &Vec<Sample>) -> Sample;
}

// TODO 定数を ModDL と共通化
fn bool_to_sample(b: bool) -> Sample { if b { 1f32 } else { -1f32 } }
fn sample_to_bool(s: Sample) -> bool { s > 0f32 }
fn bool_binary(lhs: Sample, rhs: Sample, op: fn (lhs: bool, rhs: bool) -> bool) -> Sample {
	bool_to_sample(op(sample_to_bool(lhs), sample_to_bool(rhs)))
}

macro_rules! unary_calc {
	($name: ident, $calc: expr) => {
		pub struct $name { }
		impl Calc for $name {
			fn arg_count() -> i32 { 1 }
			fn calc(args: &Vec<Sample>) -> Sample { $calc(args[0]) }
		}
	}
}

macro_rules! binary_calc {
	($name: ident, $calc: expr) => {
		pub struct $name { }
		impl Calc for $name {
			fn arg_count() -> i32 { 2 }
			fn calc(args: &Vec<Sample>) -> Sample { $calc(args[0], args[1]) }
		}
	}
}

 ////
//// arithmetic operations

binary_calc!(AddCalc, |lhs, rhs| lhs + rhs);
binary_calc!(SubCalc, |lhs, rhs| lhs - rhs);
binary_calc!(MulCalc, |lhs, rhs| lhs * rhs);
binary_calc!(DivCalc, |lhs, rhs| lhs / rhs);
binary_calc!(RemCalc, |lhs, rhs| lhs % rhs);
binary_calc!(PowCalc, |lhs: Sample, rhs| lhs.powf(rhs));

 ////
//// comparison operations

binary_calc!(LtCalc, |lhs, rhs| bool_to_sample(lhs < rhs));
binary_calc!(LeCalc, |lhs, rhs| bool_to_sample(lhs <= rhs));
binary_calc!(EqCalc, |lhs, rhs| bool_to_sample(lhs == rhs));
binary_calc!(NeCalc, |lhs, rhs| bool_to_sample(lhs != rhs));
binary_calc!(GtCalc, |lhs, rhs| bool_to_sample(lhs > rhs));
binary_calc!(GeCalc, |lhs, rhs| bool_to_sample(lhs >= rhs));

 ////
//// logical operations

binary_calc!(AndCalc, |lhs, rhs| bool_binary(lhs, rhs, |lhs, rhs| lhs && rhs));
binary_calc!(OrCalc, |lhs, rhs| bool_binary(lhs, rhs, |lhs, rhs| lhs || rhs));

 ////
//// functions

unary_calc!(LogCalc, |arg: Sample| arg.ln());
