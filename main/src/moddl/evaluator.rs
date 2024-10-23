use super::{
	console::warn, error::*, import::ImportCache, lambda_function::*, scope::*, value::*
};

extern crate parser;
use parser::{moddl::ast::*, common::Location};

use crate::{
	calc::*,
};

use std::{
	cell::RefCell,
	collections::{hash_map::HashMap, HashSet},
	rc::Rc,
};

pub fn evaluate(expr: &Expr, vars: &Rc<RefCell<Scope>>, imports: &mut ImportCache) -> ModdlResult<Value> {
// dbg!(expr as *const Expr);
	let body = match &expr.body {
		ExprBody::Connect { lhs, rhs } => {
			let (l_str, _) = evaluate(lhs, vars, imports)?.as_node_structure() ?;
			let (r_str, _) = evaluate(rhs, vars, imports)?.as_node_structure() ?;
			Ok(ValueBody::NodeStructure(NodeStructure::Connect(Box::new(l_str), Box::new(r_str))))
		},

		ExprBody::Power { lhs, rhs } => evaluate_binary_structure::<PowCalc>(lhs, rhs, vars, imports),
		ExprBody::Multiply { lhs, rhs } => evaluate_binary_structure::<MulCalc>(lhs, rhs, vars, imports),
		ExprBody::Divide { lhs, rhs } => evaluate_binary_structure::<DivCalc>(lhs, rhs, vars, imports),
		ExprBody::Remainder { lhs, rhs } => evaluate_binary_structure::<RemCalc>(lhs, rhs, vars, imports),
		ExprBody::Add { lhs, rhs } => evaluate_binary_structure_overloaded::<AddCalc>(lhs, rhs, vars, imports, overload_add),
		ExprBody::Subtract { lhs, rhs } => evaluate_binary_structure::<SubCalc>(lhs, rhs, vars, imports),
		ExprBody::Less { lhs, rhs } => evaluate_binary_structure::<LtCalc>(lhs, rhs, vars, imports),
		ExprBody::LessOrEqual { lhs, rhs } => evaluate_binary_structure::<LeCalc>(lhs, rhs, vars, imports),
		ExprBody::Equal { lhs, rhs } => evaluate_binary_structure::<EqCalc>(lhs, rhs, vars, imports),
		ExprBody::NotEqual { lhs, rhs } => evaluate_binary_structure::<NeCalc>(lhs, rhs, vars, imports),
		ExprBody::Greater { lhs, rhs } => evaluate_binary_structure::<GtCalc>(lhs, rhs, vars, imports),
		ExprBody::GreaterOrEqual { lhs, rhs } => evaluate_binary_structure::<GeCalc>(lhs, rhs, vars, imports),
		ExprBody::And { lhs, rhs } => evaluate_binary_structure::<AndCalc>(lhs, rhs, vars, imports),
		ExprBody::Or { lhs, rhs } => evaluate_binary_structure::<OrCalc>(lhs, rhs, vars, imports),

		ExprBody::Not { arg } => evaluate_unary_structure::<NotCalc>(arg, vars, imports),
		ExprBody::Negate { arg } => evaluate_unary_structure::<NegCalc>(arg, vars, imports),
		ExprBody::Plus { arg } => evaluate_unary_structure::<PlusCalc>(arg, vars, imports),

		ExprBody::Identifier(id) => {
			let (val, _) = vars.borrow().lookup(id).ok_or_else(|| { error(ErrorType::VarNotFound { var: id.clone() }, expr.loc.clone()) }) ?;
			Ok(val.clone())
		},
		ExprBody::IdentifierLiteral(id) => Ok(ValueBody::IdentifierLiteral(id.clone())),
		ExprBody::StringLiteral(content) => Ok(ValueBody::String(content.clone())),
		ExprBody::ArrayLiteral(content) => {
			// TODO map() を使いたいがクロージャで ? を使っているとうまくいかず。いい書き方があれば修正
			let mut result = vec![];
			for elem in content {
				result.push(evaluate(&*elem, vars, imports) ?);
			}
			Ok(ValueBody::Array(result))
		},
		ExprBody::AssocLiteral(content) => {
			// TODO map() を使いたいがクロージャで ? を使っているとうまくいかず。いい書き方があれば修正
			let mut result = HashMap::<String, Value>::with_capacity(content.len());
			for (key, value_expr) in content {
				result.insert(key.clone(), evaluate(&*value_expr, vars, imports) ?);
			}
			Ok(ValueBody::Assoc(result))
		},
		ExprBody::Condition { cond, then, els } => evaluate_conditional_expr(cond, then, els, vars, imports),
		ExprBody::LambdaFunction { params, body } => {
			let mut param_values: Vec<Param> = vec![];
			params.iter().try_for_each(|param| {
				param_values.push(Param {
					name: param.name.clone(),
					default: match &param.default { // param.default.map で書きたいがうまくいかず
						None => None,
						Some(default) => Some(evaluate(&*default, vars, imports) ?),
					},
				});
				ModdlResult::Ok(())
			}) ?;
			// Value 側で式を使う必要があるので、単純に式を clone して持たせておく。
			// 何とかして参照した方が効率的だが
			Ok(ValueBody::Function(Rc::new(LambdaFunction::new(param_values, *body.clone(), vars))))
		}
		ExprBody::LambdaNode { input_param, body } => {
			let vars = Scope::child_of(vars.clone());
			vars.borrow_mut().set(input_param,
					(ValueBody::NodeStructure(NodeStructure::Placeholder { name: input_param.clone() }), expr.loc.clone())) ?;
			let result = Ok(ValueBody::NodeStructure(NodeStructure::Lambda {
				input_param: input_param.clone(),
				body: Box::new(evaluate(body, &vars, imports)?.as_node_structure()?.0), // TODO loc も引き渡す
			}));
			result
		},
		// Expr::ModuleParamExpr { module_def, label: String, ctor_params: AssocArray, signal_params: AssocArray } => {}
		ExprBody::FloatLiteral(value) => Ok(ValueBody::Float(*value)),
		ExprBody::TrackSetLiteral(tracks) => Ok(ValueBody::TrackSet(tracks.clone())),
		// Expr::MmlLiteral(String) => {}
		// Expr::AssocArrayLiteral(AssocArray) => {}
		ExprBody::FunctionCall { function, args } => {
			let (function, _) = evaluate(function, vars, imports)?.as_function() ?;

			let arg_names = function.signature().iter().map(|name| name.to_string()).collect();
			let resolved_args = resolve_args(&arg_names, args, &expr.loc) ?;
			let mut value_args = HashMap::new();
			// TODO map() を使いたいがクロージャで ? を使っているとうまくいかず。いい書き方があれば修正
			for (name, expr) in &resolved_args {
				value_args.insert(name.clone(), evaluate(expr, vars, imports) ?);
			}

			function.call(&value_args, &vars, expr.loc.clone(), imports).map(|(v, _)| v)
		},
		ExprBody::PropertyAccess { assoc, name } => {
			let assoc_val = evaluate(assoc, vars, imports) ?;
			let (assoc, _) = assoc_val.as_assoc() ?;
			let val = assoc.get(name);
			Ok(val.map(|(v, _)| v).ok_or_else(|| error(ErrorType::EntryNotFound { name: name.clone() }, expr.loc.clone()))?.clone())
		},
		ExprBody::NodeWithArgs { node_def, /* label, */ args } => {
			let (factory, _) = evaluate(node_def, vars, imports)?.as_node_factory() ?;

			let arg_names = factory.node_arg_specs().iter().map(|spec| spec.name.clone()).collect();
			let resolved_args = resolve_args(&arg_names, args, &expr.loc) ?;

			// TODO map() を使いたいがクロージャで ? を使っているとうまくいかず。いい書き方があれば修正
			let mut value_args = HashMap::new();
			for (name, expr) in &resolved_args {
				value_args.insert(name.clone(), evaluate(expr, vars, imports) ?);
			}

			Ok(ValueBody::NodeStructure(NodeStructure::NodeCreation {
				factory,
				label: None,
				args: value_args,
			}))
		},

		ExprBody::MmlLiteral(_) => unimplemented!(),

		ExprBody::Labeled { label, inner } => {
			let (inner_val, _) = evaluate(inner, vars, imports) ?;

			let warn_ineffective_label = || warn(format!("ineffective label \"{}\" ignored at {}", &label.0, &expr.loc));

			// ラベルをつけれる対象は、数値定数、引数なしの NodeFactory、引数ありの NodeFactory（NodeCreation）の 3 つ。
			// 上記の値にラベルをつけると、結果は必ず NodeStructure になる。
			// 上記以外にラベルをつけるのは無意味であり、警告とともにラベルは無視される
			// 数値定数はラベルをつけると NodeStructure になるので、表現としては Float と NodeStructure::Constant の 2 通りある。
			// すでにラベルがついている値にさらにラベルをつけるのは問題ない
			let new_val = match inner_val {
				ValueBody::Float(value) => ValueBody::NodeStructure(
					NodeStructure::Constant { value, label: Some(label.clone()) },
				),
				ValueBody::NodeFactory(factory) => ValueBody::NodeStructure(
					NodeStructure::NodeCreation { factory, args: HashMap::new(), label: Some(label.clone()) },
				),
				ValueBody::NodeStructure(strukt) => ValueBody::NodeStructure(
					match strukt {
						NodeStructure::Constant { value, label: _ } => NodeStructure::Constant { value, label: Some(label.clone()) },
						NodeStructure::NodeCreation { factory, args, label: _ } => NodeStructure::NodeCreation { factory, args, label: Some(label.clone()) },
						_ => {
							warn_ineffective_label();
							strukt
						},
					},
				),
				_ => {
					warn_ineffective_label();
					inner_val
				}
			};

			Ok(new_val)
		},

		ExprBody::LabelFilter { strukt, filter } => {
			let (struct_val, struct_loc) = evaluate(strukt, vars, imports)?.as_node_structure() ?;
			let filter = build_label_filter(filter, &struct_loc) ?;

			Ok(ValueBody::NodeStructure(filter_labels(unguard_labels(&struct_val), &struct_loc, &filter) ?))
		},
		ExprBody::LabelPrefix { strukt, prefix } => {
			let (struct_val, struct_loc) = evaluate(strukt, vars, imports)?.as_node_structure() ?;

			Ok(ValueBody::NodeStructure(add_prefix_to_labels(unguard_labels(&struct_val), &struct_loc, prefix.0.as_str()) ?))
		},
	} ?;
	Ok((body, expr.loc.clone()))
}

fn unguard_labels(strukt: &NodeStructure) -> &NodeStructure {
	// LabelGuard だったら開封する
	match strukt {
		NodeStructure::LabelGuard(inner) => inner,
		_ => strukt,
	}
}

fn filter_labels(strukt: &NodeStructure, loc: &Location, filter: &LabelFilter) -> ModdlResult<NodeStructure> {
	let transform_label = |label: &Option<QualifiedLabel>| match label {
		None => None,
		Some(label) => {
			if let Some(new_label) = filter.renames.get(label) {
				Some(new_label.clone())
			} else {
				let expected_contains = matches!(filter.list_type, ListType::Allow);
				if filter.list.contains(label) == expected_contains { Some(label.clone()) } else { None }
			}
		}
	};

	transform_labels(strukt, loc, &transform_label)
}

fn add_prefix_to_labels(strukt: &NodeStructure, loc: &Location, prefix: &str) -> ModdlResult<NodeStructure> {
	let transform_label = |label: &Option<QualifiedLabel>| match label {
		None => None,
		Some(label) => { Some(QualifiedLabel(format!("{}.{}", prefix, label.0))) },
	};

	transform_labels(strukt, loc, &transform_label)
}

fn transform_labels<F>(strukt: &NodeStructure, loc: &Location, transform_label: &F) -> ModdlResult<NodeStructure>
where F: Fn (&Option<QualifiedLabel>) -> Option<QualifiedLabel> {
	let recurse = |strukt| transform_labels(strukt, loc, transform_label);

	match strukt {
		NodeStructure::Calc { node_factory, args } => Ok(NodeStructure::Calc {
			node_factory: node_factory.clone(),
			args: {
				let results: ModdlResult<Vec<_>> = args.iter().map(|arg| recurse(arg)).collect();
				results?.into_iter().map(Box::new).collect()
			},
		}),
		NodeStructure::Connect(lhs, rhs) => Ok(NodeStructure::Connect(
			Box::new(recurse(lhs) ?),
			Box::new(recurse(rhs) ?),
		)),
		NodeStructure::Condition { cond, then, els } => Ok(NodeStructure::Condition {
			cond: Box::new(recurse(cond) ?),
			then: Box::new(recurse(then) ?),
			els: Box::new(recurse(els) ?),
		}),
		NodeStructure::Lambda { input_param, body } => Ok(NodeStructure::Lambda {
			input_param: input_param.clone(),
			body: Box::new(recurse(body) ?),
		}),
		NodeStructure::NodeCreation { factory, args, label } => Ok(NodeStructure::NodeCreation {
			factory: factory.clone(),
			args: args.keys().map(|arg_name| {
				let (value, value_loc) = &args[arg_name];
				let new_value = match value {
					ValueBody::NodeStructure(strukt) => ValueBody::NodeStructure(recurse(strukt) ?),
					_ => value.clone(),
				};
				Ok((arg_name.clone(), (new_value, value_loc.clone())))
			}).collect::<ModdlResult<HashMap<String, Value>>>() ?,
			label: transform_label(label),
		}),
		NodeStructure::Constant { value, label } => Ok(NodeStructure::Constant {
			value: *value,
			label: transform_label(label),
		}),
		NodeStructure::Placeholder { .. }
		| NodeStructure::LabelGuard(..) // この中のラベルは無視するため、何もしない
		=> Ok(strukt.clone()),
	}
}

#[derive(Debug)]
enum ListType { Allow, Deny }
#[derive(Debug)]
struct LabelFilter {
	list_type: ListType,
	list: HashSet<QualifiedLabel>,
	renames: HashMap<QualifiedLabel, QualifiedLabel>,
}
fn build_label_filter(specs: &Vec<LabelFilterSpec>, loc: &Location) -> ModdlResult<LabelFilter> {
	validate_label_filter_specs(specs, loc) ?;

	let list_type = if specs.iter().any(|spec| matches!(spec, LabelFilterSpec::Allow(_)))
			|| specs.iter().all(|spec| matches!(spec, LabelFilterSpec::Rename(..))) {
		ListType::Allow
	} else {
		ListType::Deny
	};

	let list = match list_type {
		ListType::Allow => {
			specs.iter().filter_map(|spec| match spec {
				LabelFilterSpec::Allow(label) => Some(label.clone()),
				LabelFilterSpec::Rename(label, _) => Some(label.clone()),
				_ => None,
			}).collect()
		},
		ListType::Deny => {
			specs.iter().filter_map(|spec| match spec {
				LabelFilterSpec::Deny(label) => Some(label.clone()),
				_ => None,
			}).collect()
		},
	};
	let renames = specs.iter().filter_map(|spec| match spec {
		LabelFilterSpec::Rename(before, after) => Some((before.clone(), after.clone())),
		_ => None,
	}).collect();

	Ok(LabelFilter {
		list_type,
		list,
		renames,
	})
}

fn validate_label_filter_specs(specs: &Vec<LabelFilterSpec>, loc: &Location) -> ModdlResult<()> {
	let make_error = || Err(error(ErrorType::LabelFilterInconsistent, loc.clone()));

	/// これらのモードのうち、どれかに該当すれば OK（判定において Rename は関係ないので無視する）
	enum Mode {
		/// ワイルドカード 1 つしかない
		AllowAll,
		/// 1 つ以上の許可しかない
		Allow,
		/// 1 つ以上の拒否しかない
		Deny,
	}
	let mut mode = None;
	let mut rename_exists = false;
	for spec in specs {
		match spec {
			LabelFilterSpec::AllowAll => {
				match mode {
					None => { mode = Some(Mode::AllowAll); },
					Some(_) => make_error() ?,
				}
			},
			LabelFilterSpec::Allow(_) => {
				match mode {
					None => { mode = Some(Mode::Allow); },
					Some(Mode::Allow) => (),
					Some(_) => make_error() ?,
				}
			}
			LabelFilterSpec::Deny(_) => {
				match mode {
					None => { mode = Some(Mode::Deny); },
					Some(Mode::Deny) => (),
					Some(_) => make_error() ?,
				}
			}
			LabelFilterSpec::Rename(..) => {
				rename_exists = true;
			}
		}
	}
	if mode.is_none() && ! rename_exists { make_error() ?; }

	// ラベルがかぶってたらエラーにする
	let mut uniq = HashSet::<QualifiedLabel>::new();
	let mut check_uniqueness = |label| if uniq.contains(label) {
		make_error()
	} else {
		uniq.insert(label.clone());
		Ok(())
	};
	for spec in specs {
		match spec {
			LabelFilterSpec::Allow(label)
			| LabelFilterSpec::Deny(label)
			| LabelFilterSpec::Rename(label, _) => check_uniqueness(label) ?,
			_ => { },
		}
	}

	Ok(())
}


fn evaluate_unary_structure<C: Calc + 'static>(
	arg: &Expr,
	vars: &Rc<RefCell<Scope>>,
	imports: &mut ImportCache,
) -> ModdlResult<ValueBody> {
	let arg_val = evaluate(arg, vars, imports) ?;

	// ラベルのついていない定数はコンパイル時に計算する。
	// ラベルがついた定数（NodeStructure になる）は演奏中の設定の対象になるため対象外
	if let Some(arg_float) = arg_val.0.as_float() {
		return Ok(ValueBody::Float(C::calc(&vec![arg_float])));
	}

	let (arg_str, _) = arg_val.as_node_structure() ?;
	Ok(ValueBody::NodeStructure(NodeStructure::Calc {
		node_factory: Rc::new(CalcNodeFactory::<C>::new()),
		args: vec![Box::new(arg_str)],
	}))
}

fn overload_add(lhs: &ValueBody, rhs: &ValueBody) -> Option<ModdlResult<ValueBody>> {
	match (lhs, rhs) {
		(ValueBody::String(lhs), rhs) => {
			let result = rhs.to_str(|rhs| lhs.clone() + rhs);
			Some(Ok(ValueBody::String(result)))
		},
		(lhs, ValueBody::String(rhs)) => {
			let result = lhs.to_str(|lhs| lhs.to_string() + rhs);
			Some(Ok(ValueBody::String(result)))
		},
		(ValueBody::Array(lhs), ValueBody::Array(rhs)) => {
			let mut result = lhs.clone();
			for elem in rhs {
				result.push(elem.clone());
			}
			Some(Ok(ValueBody::Array(result)))
		},
		(ValueBody::Assoc(lhs), ValueBody::Assoc(rhs)) => {
			let mut result = lhs.clone();
			for (k, v) in rhs {
				result.insert(k.clone(), v.clone());
			}
			Some(Ok(ValueBody::Assoc(result)))
		},
		_ => None,
	}
}

fn evaluate_binary_structure<C: Calc + 'static>(
	lhs: &Expr,
	rhs: &Expr,
	vars: &Rc<RefCell<Scope>>,
	imports: &mut ImportCache,
) -> ModdlResult<ValueBody> {
	evaluate_binary_structure_overloaded::<C>(lhs, rhs, vars, imports, |_, _| None)
}

fn evaluate_binary_structure_overloaded<C: Calc + 'static>(
	lhs: &Expr,
	rhs: &Expr,
	vars: &Rc<RefCell<Scope>>,
	imports: &mut ImportCache,
	overload: fn (&ValueBody, &ValueBody) -> Option<ModdlResult<ValueBody>>,
) -> ModdlResult<ValueBody> {
	let ref l_val @ (ref l_body, _) = evaluate(lhs, vars, imports) ?;
	let ref r_val @ (ref r_body, _) = evaluate(rhs, vars, imports) ?;

	if let Some(result) = overload(l_body, r_body) {
		return Ok(result ?);
	}

	// ラベルのついていない定数はコンパイル時に計算する。
	// ラベルがついた定数（NodeStructure になる）は演奏中の設定の対象になるため対象外
	if let (Some(l_float), Some(r_float)) = (l_body.as_float(), r_body.as_float()) {
		return Ok(ValueBody::Float(C::calc(&vec![l_float, r_float])));
	}

	let (l_str, _) = l_val.as_node_structure() ?;
	let (r_str, _) = r_val.as_node_structure() ?;
	Ok(ValueBody::NodeStructure(NodeStructure::Calc {
		node_factory: Rc::new(CalcNodeFactory::<C>::new()),
		args: vec![Box::new(l_str), Box::new(r_str)],
	}))
}

fn evaluate_conditional_expr(cond: &Expr, then: &Expr, els: &Expr, vars: &Rc<RefCell<Scope>>, imports: &mut ImportCache) -> ModdlResult<ValueBody> {
	// cond が定数式の場合は短絡評価する。
	// 式全体が定数式になるかどうかは、評価する方の枝の評価結果が定数式になるかどうかに拠る
	let ref cond_val @ (ref cond_body, _) = evaluate(cond, vars, imports) ?;
	if let Some(cond_bool) = cond_body.as_boolean() {
		return if cond_bool {
			evaluate(then, vars, imports).map(|(v, _)| v)
		} else {
			evaluate(els, vars, imports).map(|(v, _)| v)
		};
	}

	// cond が定数式でない場合は NodeStructure として演奏時に評価する。
	// then と else も NodeStructure でなければならないので、定数式にはならない
	let then_val = evaluate(then, vars, imports) ?;
	let else_val = evaluate(els, vars, imports) ?;
	let (cond_str, _) = cond_val.as_node_structure() ?;
	let (then_str, _) = then_val.as_node_structure() ?;
	let (else_str, _) = else_val.as_node_structure() ?;
	Ok(ValueBody::NodeStructure(NodeStructure::Condition {
		cond: Box::new(cond_str),
		then: Box::new(then_str),
		els: Box::new(else_str),
	}))
}

/**
 * 引数名を省略した実引数を、要求された引数リストと照合して解決することで、全ての引数を「引数名」と「式」の対応関係にする。
 * 引数の重複もチェックする（引数名が省略されていても解決してからチェックする）
 */
fn resolve_args<'a>(arg_names: &'a Vec<String>, args: &'a Args, expr_loc: &Location) -> ModdlResult<HashMap<String, &'a Box<Expr>>> {
	let mut result = HashMap::<String, &'a Box<Expr>>::new();
	let mut add = |name: &String, expr: &'a Box<Expr>| -> ModdlResult<()> {
		if result.contains_key(name) { return Err(error(ErrorType::EntryDuplicate { name: name.clone() }, expr_loc.clone())); }

		result.insert(name.clone(), expr);
		Ok(())
	};

	// TODO 無名引数の過剰チェックをするなら未知の引数もエラーにする。しないならしない。で統一する
	// if args.unnamed.len() > arg_names.len() { return Err(Error::TooManyUnnamedArgs) };

	let mut unnamed_args = arg_names.iter().zip(& args.unnamed);
	unnamed_args.try_for_each(|(name, arg)| add(name, arg)) ?;
	args.named.iter().try_for_each(|(name, arg)| add(name, arg)) ?;

	Ok(result)
}
