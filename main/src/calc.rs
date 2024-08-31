use crate::core::common::Sample;
pub trait BinaryOp {
	fn oper(lhs: Sample, rhs: Sample) -> Sample;
}

pub trait Calc {
	fn operator() -> &'static str;
	fn arg_count() -> i32;
	fn calc(args: &Vec<Sample>) -> Sample;
}

// TODO 定数を ModDL と共通化
fn bool_to_sample(b: bool) -> Sample { if b { 1f32 } else { -1f32 } }
pub fn sample_to_bool(s: Sample) -> bool { s > 0f32 }
fn bool_binary(lhs: Sample, rhs: Sample, op: fn (lhs: bool, rhs: bool) -> bool) -> Sample {
	bool_to_sample(op(sample_to_bool(lhs), sample_to_bool(rhs)))
}

macro_rules! unary_calc {
	($name: ident, $operator: expr, $calc: expr) => {
		pub struct $name { }
		impl Calc for $name {
			fn operator() -> &'static str { $operator }
			fn arg_count() -> i32 { 1 }
			fn calc(args: &Vec<Sample>) -> Sample { $calc(args[0]) }
		}
	}
}

macro_rules! binary_calc {
	($name: ident, $operator: expr, $calc: expr) => {
		pub struct $name { }
		impl Calc for $name {
			fn operator() -> &'static str { $operator }
			fn arg_count() -> i32 { 2 }
			fn calc(args: &Vec<Sample>) -> Sample { $calc(args[0], args[1]) }
		}
	}
}

 ////
//// arithmetic operations

// ここに記載する演算子の文字列は to_string() で使われる。
// parser.rs のものと一致させること

binary_calc!(AddCalc, "+", |lhs, rhs| lhs + rhs);
binary_calc!(SubCalc, "-", |lhs, rhs| lhs - rhs);
binary_calc!(MulCalc, "*", |lhs, rhs| lhs * rhs);
binary_calc!(DivCalc, "/", |lhs, rhs| lhs / rhs);
binary_calc!(RemCalc, "%", |lhs, rhs| lhs % rhs);
binary_calc!(PowCalc, "^", |lhs: Sample, rhs| lhs.powf(rhs));

 ////
//// comparison operations

binary_calc!(LtCalc, "<", |lhs, rhs| bool_to_sample(lhs < rhs));
binary_calc!(LeCalc, "<=", |lhs, rhs| bool_to_sample(lhs <= rhs));
binary_calc!(EqCalc, "==", |lhs, rhs| bool_to_sample(lhs == rhs));
binary_calc!(NeCalc, "!=", |lhs, rhs| bool_to_sample(lhs != rhs));
binary_calc!(GtCalc, ">", |lhs, rhs| bool_to_sample(lhs > rhs));
binary_calc!(GeCalc, ">=", |lhs, rhs| bool_to_sample(lhs >= rhs));

 ////
//// logical operations

binary_calc!(AndCalc, "&&", |lhs, rhs| bool_binary(lhs, rhs, |lhs, rhs| lhs && rhs));
binary_calc!(OrCalc, "||", |lhs, rhs| bool_binary(lhs, rhs, |lhs, rhs| lhs || rhs));

 ////
//// functions

unary_calc!(NegCalc, "-", |arg: Sample| -arg);
unary_calc!(LogCalc, "log", |arg: Sample| arg.ln());
unary_calc!(Log10Calc, "log10", |arg: Sample| arg.log(10f32));
unary_calc!(SinCalc, "sin", |arg: Sample| arg.sin());
unary_calc!(CosCalc, "cos", |arg: Sample| arg.cos());
unary_calc!(TanCalc, "tan", |arg: Sample| arg.tan());
unary_calc!(AbsCalc, "abs", |arg: Sample| arg.abs());
unary_calc!(SignumCalc, "signum", |arg: Sample| arg.signum());
unary_calc!(FloorCalc, "floor", |arg: Sample| arg.floor());
unary_calc!(CeilCalc, "ceil", |arg: Sample| arg.ceil());
unary_calc!(RoundCalc, "round", |arg: Sample| arg.round());
unary_calc!(TruncCalc, "trunc", |arg: Sample| arg.trunc());
