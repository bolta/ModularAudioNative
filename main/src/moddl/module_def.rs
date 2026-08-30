use parser::moddl::ast::QualifiedLabel;

use crate::calc::*;
use crate::core::common::*;
use crate::core::node::Node;
use crate::core::node_factory::NodeFactory;
use crate::moddl::domain::DomainHint;
use crate::moddl::value::Value;
use crate::node::arith::*;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::rc::Rc;
use itertools::Itertools;

/// 生成すべき Node の構造を表現する型。
/// Value から直接 Node を生成すると問題が多いので、一旦この形式を挟む
#[derive(Clone)]
pub enum ModuleDef {
	Calc{ node_factory: Rc<dyn CalcNodeFactoryTrait>, args: Vec<Box<ModuleDef>> },
	Connect(Box<ModuleDef>, Box<ModuleDef>),
	Condition { cond: Box<ModuleDef>, then: Box<ModuleDef>, els: Box<ModuleDef> },
	Lambda { input_param: String, body: Box<ModuleDef> },
	NodeCreation {
		factory: NodeDef,
		args: HashMap<String, Value>,
		label: Option<QualifiedLabel>,
	},
	Constant {
		value: f32,
		label: Option<QualifiedLabel>,
	},
	Placeholder { name: String },
	LabelGuard(Box<ModuleDef>),
}
impl ModuleDef {
	pub fn label(&self) -> Option<QualifiedLabel> {
		match self {
			ModuleDef::NodeCreation { label, .. } | ModuleDef::Constant { label, .. } => label.clone(),
			_ => None,
		}
	}

	pub fn to_string(&self) -> String {
		match self {
			ModuleDef::Calc{ node_factory, args } => {
				match args.len() {
					2 => format!("({} {} {})", args[0].to_string(), node_factory.operator(), args[1].to_string()),
					_ => {
						// Calc に 3 項以上のものはないので、ここで処理するのは単項演算だけのはずだが、
						// もし 3 項以上あった場合は全ての引数を列挙する
						let content = args.iter().map(|a| a.to_string()).join(", ");
						format!("{}({})", node_factory.operator(), content)
					},
				}
			},
			Self::Connect(lhs, rhs) => format!("({} | {})", lhs.to_string(), rhs.to_string()),
			Self::Condition { cond, then, els } => format!("(if {} then {} else {})", cond.to_string(), then.to_string(), els.to_string()),
			Self::Lambda { input_param, body } => format!("(={}=> {})", input_param, body.to_string()),
			Self::NodeCreation { factory: _, args, label } => {
				// TODO NodeDef には名前をつけたい
				// TODO その他の情報もなるべく出したい
				let factory_str = "(NodeDef)";
				let args_str = match args.len() {
					0 => "".to_string(),
					_ => {
						let content = args.iter().map(|(k, (v, _))| format!("{}: {}", k, v.force_to_string())).join(", ");
						format!("{{ {} }}", content)
					},
				};
				let label_str = match label {
					None => "".to_string(),
					Some(label) => format!("@{}", label),
				};
				format!("{}{}{}", factory_str, args_str, label_str)
			},
			Self::Constant { value, label } => match label {
				None => value.to_string(),
				Some(label) => format!("{}@{}", value, label),
			},
			Self::Placeholder { name } => format!("Placeholder({})", name),
			Self::LabelGuard(content) => format!("LabelGuard({})", content.to_string()),
		}
	}
}

pub trait CalcNodeFactoryTrait {
	fn operator(&self) -> &str;
	fn create_mono(&self, args: Vec<MonoNodeIndex>) -> Box<dyn Node>;
	fn create_stereo(&self, args: Vec<StereoNodeIndex>) -> Box<dyn Node>;
}
// #[derive(Clone)]
pub struct CalcNodeFactory<C: 'static + Calc> {
	_c: PhantomData<fn () -> C>,
}
impl <C: 'static + Calc> CalcNodeFactory<C> {
	pub fn new() -> Self { Self { _c: PhantomData } }
}
impl <C: 'static + Calc> CalcNodeFactoryTrait for CalcNodeFactory<C> {
	fn operator(&self) -> &str { C::operator() }
	fn create_mono(&self, args: Vec<MonoNodeIndex>) -> Box<dyn Node> {
		Box::new(MonoCalc::<C>::new(args))
	}
	fn create_stereo(&self, args: Vec<StereoNodeIndex>) -> Box<dyn Node> {
		Box::new(StereoCalc::<C>::new(args))
	}
}

#[derive(Clone)]
pub struct NodeDef {
	pub node: Rc<dyn NodeFactory>,

	// 検討メモ：定義域の情報は NodeFactory に持たせるのが筋かもしれない。
	// ただ、PulseOsc::duty とかは 0 <= x <= 1 で誰も異論はなさそうだが、
	// adsrEnv::attack なんかだと 0 <= x は確実としても、上限はモデル的には存在しないので、NodeFactory に置くのは難しそう。
	// 一方、UI 的には何かしらの上限を決めないといけない。ということは UI に置くべきだとなるが、
	// 0 <= x はモデルに置くのが妥当そうでもある…
	// とりあえずは全て UI に置いて進める
	// pub ui: Option<UiDef>,
	// pub controls: Vec<ControlDef>,
	pub domains: HashMap<String, DomainHint>,
}
impl NodeDef {
	pub fn without_domain(node: Rc<dyn NodeFactory>) -> Self {
		Self { node, domains: HashMap::from([]) }
	}
}
