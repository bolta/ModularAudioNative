use super::{
	ast::*,
	value::*,
};

pub fn evaluate(expr: &Expr /* context */) -> Value {
	match expr {
		Expr::Connect { lhs, rhs } => evaluate_binary_structure(lhs, rhs, NodeStructure::Connect),
		Expr::Power { lhs, rhs } => evaluate_binary_structure(lhs, rhs, NodeStructure::Power),
		Expr::Multiply { lhs, rhs } => evaluate_binary_structure(lhs, rhs, NodeStructure::Multiply),
		Expr::Divide { lhs, rhs } => evaluate_binary_structure(lhs, rhs, NodeStructure::Divide),
		Expr::Remainder { lhs, rhs } => evaluate_binary_structure(lhs, rhs, NodeStructure::Remainder),
		Expr::Add { lhs, rhs } => evaluate_binary_structure(lhs, rhs, NodeStructure::Add),
		Expr::Subtract { lhs, rhs } => evaluate_binary_structure(lhs, rhs, NodeStructure::Subtract),
		Expr::Identifier(id) => Value::Identifier(id.clone()),
		Expr::Lambda { input_param: String, body } => unimplemented!(),
		// Expr::ModuleParamExpr { module_def, label: String, ctor_params: AssocArray, signal_params: AssocArray } => {}
		Expr::FloatLiteral(value) => Value::Float(*value),
		Expr::TrackSetLiteral(tracks) => Value::TrackSet(tracks.clone()),
		// Expr::MmlLiteral(String) => {}
		// Expr::AssocArrayLiteral(AssocArray) => {}
		Expr::NodeWithArgs { node_def, label, args } => Value::NodeStructure(NodeStructure::NodeWithArgs {
			factory: Box::new(evaluate_as_node_structure(node_def)),
			label: label.clone(),
			args: args.iter().map(|(name, expr)| (name.clone(), evaluate(expr))).collect(),
		}),

		_ => unimplemented!()
	}
}

fn evaluate_binary_structure(lhs: &Expr, rhs: &Expr, make_structure: fn (Box<NodeStructure>, Box<NodeStructure>) -> NodeStructure) -> Value {
	let l_str = evaluate_as_node_structure(lhs);
	let r_str = evaluate_as_node_structure(rhs);
	Value::NodeStructure(make_structure(Box::new(l_str), Box::new(r_str)))
}
fn evaluate_as_node_structure(expr: &Expr) -> NodeStructure {
	// TODO エラー処理
	evaluate(expr).as_node_structure().unwrap()
}