use std::{cell::RefCell, collections::HashMap, path::{Path, PathBuf}, rc::Rc};

use parser::common::Location;

use crate::wave::waveform_host::WaveformHost;

use super::{common::read_file, error::{error, ErrorType, ModdlResult}, executor::process_statements, path::resolve_path, scope::Scope, value::{NodeStructure, Value, ValueBody}};

pub struct ImportCache<'a> {
	// TODO String じゃなくて Path とか他の型になるのかも？
	imports: HashMap<PathBuf, Value>,
	pub waveforms: &'a mut WaveformHost,
}
impl <'a> ImportCache<'a> {
	pub fn new(waveforms: &'a mut WaveformHost) -> Self {
		Self {
			imports: HashMap::new(),
			waveforms,
		}
	}

	pub fn import(&mut self, path: &Path, base_path: &Path, root_scope: Rc<RefCell<Scope>>, loc: &Location) -> ModdlResult<Value> {
		let abs_path = resolve_path(path, base_path);
		match self.imports.get(&abs_path) {
			Some(cached) => Ok(cached.clone()),
			None => {
				let moddl = read_file(abs_path.as_path()) ?;
				let pctx = process_statements(moddl.as_str(), root_scope, abs_path.as_path(), self) ?;
				match pctx.export {
					None => Err(error(ErrorType::ExportNotFound, loc.clone())),
					Some((val, loc)) => {
						// TODO @import 文ではキャッシュが効いてないっぽい…共通化する
	
						let new_val = match val {
							ValueBody::NodeStructure(strukt) => ValueBody::NodeStructure(
								match strukt {
									NodeStructure::LabelGuard(_) => {
										strukt.clone()
									},
									_ => {
										NodeStructure::LabelGuard(Box::new(strukt.clone()))
									},
								}
							),
							_ => val.clone(),
						};
						let result = (new_val, loc);
						self.imports.insert(abs_path, result.clone());
						Ok(result)
					}
				}
			}
		}
	}
}
