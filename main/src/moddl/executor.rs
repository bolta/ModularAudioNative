use super::{
	common::{make_seq_tag, read_file}, console::*, error::*, evaluator::*, import::ImportCache, io::Io, path::*, player_context::{MuteSolo, PlayerContext, TrackDef}, scope::*, value::*
};
use crate::wave::{
		wav_reader::*, waveform::Waveform, waveform_host::WaveformHost,
	};
extern crate parser;
use parser::{
	common::{Location, Span}, moddl::{ast::*, parser::compilation_unit}
};

use std::{
	cell::RefCell, collections::hash_map::HashMap, path::Path, rc::Rc
};

pub fn process_statements(moddl: &str, root_scope: Rc<RefCell<Scope>>, moddl_path: &Path, imports: &mut ImportCache) -> ModdlResult<PlayerContext> {
	let mut pctx = PlayerContext::init(moddl_path, root_scope);

	let (_, CompilationUnit { statements }) = compilation_unit()(Span::new_extra(moddl, Rc::new(moddl_path.to_path_buf())))
	.map_err(|e| error(ErrorType::Syntax(nom_error_to_owned(e)), Location::dummy())) ?;

	for stmt in &statements {
		process_statement(&stmt, &mut pctx, imports) ?;
	}

	Ok(pctx)
}

pub fn import(moddl_path: &Path, base_moddl_path: &Path, root_scope: Rc<RefCell<Scope>>, imports: &mut ImportCache) -> ModdlResult<HashMap<String, Value>> {
	let resolved_path = resolve_path(moddl_path, base_moddl_path);
	let resolved_path = resolved_path.as_path();
	let moddl = read_file(resolved_path) ?;
	let pctx = process_statements(moddl.as_str(), root_scope, resolved_path, imports) ?;

	// pctx.vars.borrow() が通らない。こう書かないといけない
	// https://github.com/rust-lang/rust/issues/41906#issuecomment-301279688
	let vars = RefCell::<Scope>::borrow(&*pctx.vars);
	Ok(vars.entries().clone())
}

fn process_statement<'a>((stmt, stmt_loc): &'a (Statement, Location), pctx: &mut PlayerContext, imports: &mut ImportCache) -> ModdlResult<()> {
	match stmt {
		Statement::Directive { name, args } => {
			match name.as_str() {
				"tempo" => {
					(*pctx).tempo = evaluate_and_perform_arg(&args, 0, &pctx.vars, stmt_loc, imports)?.as_float()?.0;
				},
				"instrument" => {
					let tracks = evaluate_and_perform_arg(&args, 0, &pctx.vars, stmt_loc, imports)?.as_track_set()?.0;
					// let instrm = & args[1];
					for track in tracks {
						let instrm = evaluate_and_perform_arg(&args, 1, &pctx.vars, stmt_loc, imports)?.as_node_structure()?.0;
						pctx.add_track_def(&track, TrackDef::Instrument(instrm), stmt_loc) ?;
						pctx.terminal_tracks.insert(track);
					}
				}
				"effect" => {
					let tracks = evaluate_and_perform_arg(&args, 0, &pctx.vars, stmt_loc, imports)?.as_track_set()?.0;
					let source_tracks = evaluate_and_perform_arg(&args, 1, &pctx.vars, stmt_loc, imports)?.as_track_set()?.0;
					let source_loc = &args[1].loc;
					// TODO source_tracks の各々が未定義ならエラーにする（循環が生じないように）

					// 定義を評価する際、source_tracks の各々を placeholder として定義しておく。
					let vars = Scope::child_of(pctx.vars.clone());
					
					for source_track in &source_tracks {
						pctx.vars.borrow_mut().set(source_track,
								(ValueBody::NodeStructure(NodeStructure::Placeholder { name: source_track.clone() }), source_loc.clone())) ?;
						pctx.terminal_tracks.remove(source_track);
					}

					let effect = evaluate_and_perform_arg(&args, 2, &vars, stmt_loc, imports)?.as_node_structure()?.0;
					for track in tracks {
						pctx.add_track_def(&track, TrackDef::Effect(source_tracks.iter().map(|t| t.clone()).collect(), effect.clone()), stmt_loc) ?;
						pctx.terminal_tracks.insert(track);
					}
				}
				"grooveCycle" => {
					(*pctx).groove_cycle = evaluate_and_perform_arg(&args, 0, &pctx.vars, stmt_loc, imports)?.as_float()?.0 as i32;
				},
				"groove" => {
					let tracks = evaluate_and_perform_arg(&args, 0, &pctx.vars, stmt_loc, imports)?.as_track_set()?.0;
					if tracks.len() != 1 { return Err(error(ErrorType::GrooveControllerTrackMustBeSingle, args[0].loc.clone())); }
					let control_track = &tracks[0];
					let target_tracks = evaluate_and_perform_arg(&args, 1, &pctx.vars, stmt_loc, imports)?.as_track_set()?.0;
					let body = evaluate_and_perform_arg(&args, 2, &pctx.vars, stmt_loc, imports)?.as_node_structure()?.0;
					pctx.add_track_def(control_track, TrackDef::Groove(body), stmt_loc) ?;
					// groove トラック自体の制御もそれ自体の groove の上で行う（even で行うことも可能だが）
					pctx.grooves.insert(control_track.clone(), (make_seq_tag(Some(&control_track), &mut pctx.seq_tags), args[1].loc.clone()));
					for track in &target_tracks {
						if let Some((_, existing_assign_loc)) = pctx.grooves.get(track) {
							return Err(error(ErrorType::GrooveTargetDuplicate {
								track: track.clone(),
								existing_assign_loc: existing_assign_loc.clone(),
								}, stmt_loc.clone()));
						}
						pctx.grooves.insert(track.clone(), (make_seq_tag(Some(&control_track), &mut pctx.seq_tags), args[1].loc.clone()));
					}
				}
				"let" => {
					let name = evaluate_and_perform_arg(&args, 0, &pctx.vars, stmt_loc, imports)?.as_identifier_literal()?.0;
					let value = evaluate_and_perform_arg(&args, 1, &mut pctx.vars, stmt_loc, imports) ?;
					pctx.vars.borrow_mut().set(&name, value) ?;
				}
				"waveform" => {
					let name = evaluate_and_perform_arg(&args, 0, &pctx.vars, stmt_loc, imports)?.as_identifier_literal()?.0;
					let (value, value_loc) = evaluate_and_perform_arg(&args, 1, &pctx.vars, stmt_loc, imports) ?;
					let waveform = if let Some(path) = value.as_string() {
						// TODO 読み込み失敗時のエラー処理
						Ok(read_wav_file(path.as_str(), None, None, None, None)
						.map_err(|e| error(e.into(), value_loc.clone())) ?)
					} else if let Some(spec) = value.as_assoc() {
						Ok(parse_waveform_spec(spec, &value_loc) ?)
					} else {
						Err(error(ErrorType::TypeMismatchAny { expected: vec![
							ValueType::String,
							ValueType::Assoc,
						]}, value_loc.clone()))
					} ?;
					let index = imports.waveforms.add(waveform);
					pctx.vars.borrow_mut().set(&name, (ValueBody::WaveformIndex(index), value_loc)) ?;
				}
				"ticksPerBar" => {
					let value = evaluate_and_perform_arg(&args, 0, &pctx.vars, stmt_loc, imports)?.as_float()?.0;
					// TODO さらに、正の整数であることを検証
					(*pctx).ticks_per_bar = value as i32;
				}
				"ticksPerBeat" => {
					let value = evaluate_and_perform_arg(&args, 0, &pctx.vars, stmt_loc, imports)?.as_float()?.0;
					// TODO さらに、正の整数であることを検証
					(*pctx).ticks_per_bar = 4 * value as i32;
				}
				"mute" => {
					let tracks = evaluate_and_perform_arg(&args, 0, &pctx.vars, stmt_loc, imports)?.as_track_set()?.0;
					set_mute_solo(MuteSolo::Mute, &tracks, pctx);
				}
				"solo" => {
					let tracks = evaluate_and_perform_arg(&args, 0, &pctx.vars, stmt_loc, imports)?.as_track_set()?.0;
					set_mute_solo(MuteSolo::Solo, &tracks, pctx);
				}
				"import" => {
					let path = evaluate_and_perform_arg(&args, 0, &pctx.vars, stmt_loc, imports)?.as_string()?.0;
					let path = Path::new(&path);
					let root_scope = pctx.vars.borrow().get_root()
							.ok_or_else(|| error(ErrorType::UnknownError { message: "cannot get root scope".to_string() }, stmt_loc.clone())) ?;
					let imported_vars = import(&path, pctx.moddl_path.as_path(), root_scope, imports) ?;
					imported_vars.iter().try_for_each(|(name, value)| {
						pctx.vars.borrow_mut().set(name, value.clone())
					}) ?;
				}
				"export" => {
					if pctx.export.is_some() {
						return Err(error(ErrorType::ExportDuplicate, stmt_loc.clone()));
					}
					pctx.export = Some(evaluate_and_perform_arg(&args, 0, &pctx.vars, stmt_loc, imports) ?);
				}
				"option" => {
					if ! pctx.allows_option_here {
						return Err(error(ErrorType::OptionNotAllowedHere, stmt_loc.clone()));
					}

					let name = evaluate_and_perform_arg(&args, 0, &pctx.vars, stmt_loc, imports)?.as_identifier_literal()?.0;
					match name.as_str() {
						"defaultLabels" => {
							pctx.use_default_labels = true;
						},
						other => {
							// 前方互換性のため警告にとどめる
							warn(format!("unknown option ignored: {}", other));
						}
					}
					// let value = evaluate_arg(&args, 1, &pctx.vars, stmt_loc);
				}
				other => {
					println!("unknown directive: {}", other);
				}
			}
		}
		Statement::Mml { tracks, mml } => {
			for track in tracks {
				if pctx.get_track_def(track).is_none() {
					return Err(error(ErrorType::TrackDefNotFound { track: track.clone() }, stmt_loc.clone()));
				}
				if let Some(mml_concat) = pctx.mmls.get_mut(track) {
					mml_concat.push_str(mml.as_str());
				} else {
					pctx.mmls.insert(track.clone(), mml.clone());
				}
			}
		}
	}

	match stmt {
		Statement::Directive { name, args: _ } if name.as_str() == "option" => { }
		_ => { 	pctx.allows_option_here = false; }
	}

	Ok(())
}

// 仕様は #16 を参照のこと
fn parse_waveform_spec(spec: &HashMap<String, Value>, loc: &Location) -> ModdlResult<Waveform> {
	let get_optional_value = |name: &str| spec.get(& name.to_string());
	let get_required_value = |name: &str| get_optional_value(name).ok_or_else(|| error(ErrorType::EntryNotFound { name: name.to_string() }, loc.clone()));

	let data_values = get_required_value("data")?.as_array()?.0;
	let sample_rate = get_required_value("sampleRate")?.as_float()?.0 as i32;
	let master_freq = get_optional_value("masterFreq").map(|value| value.as_float()).transpose()?.map(|v| v.0);
	let start_offset = get_optional_value("startOffset").map(|value| value.as_float()).transpose()?.map(|v| v.0);
	let mut end_offset =  get_optional_value("endOffset").map(|value| value.as_float()).transpose()?.map(|v| v.0);
	let mut loop_offset =  get_optional_value("loopOffset").map(|value| value.as_float()).transpose()?.map(|v| v.0);

	// TODO ステレオ対応
	let channels = 1;
	let mut data = vec![];
	for v in data_values {
		if let Ok((f, _)) = v.as_float() {
			data.push(f);
		} else if let Ok((looop, _)) = v.as_array() {
			match loop_offset {
				Some(_) => { warn("duplicate loop offset"); }, // assoc に明記されていればそちらが優先
				None => { loop_offset = Some(data.len() as f32); },
			}
			for v in looop {
				let (f, _) = v.as_float() ?;
				data.push(f);
			}
			match end_offset {
				Some(_) => { warn("duplicate end offset"); }, // assoc に明記されていればそちらが優先
				None => { end_offset = Some(data.len() as f32); },
			}
		} else {
			return Err(error(ErrorType::TypeMismatchAny { expected: vec![
				ValueType::Number,
				ValueType::Array,
			]}, v.1.clone()));
		}
	}

	Ok(Waveform::new_with_details(channels, sample_rate, data, master_freq, start_offset, end_offset, loop_offset))
}

fn evaluate_and_perform_arg(args: &Vec<Expr>, index: usize, vars: &Rc<RefCell<Scope>>, stmt_loc: &Location, imports: &mut ImportCache) -> ModdlResult<Value> {
	if index < args.len() {
		let mut value = evaluate(&args[index], vars, imports) ?;
		// while let (ValueBody::Io(io), loc) = value {
		// 	value = RefCell::<dyn Io>::borrow_mut(&io).perform(&loc) ?;
		// }
		// TODO ↑value が Labeled だったときに失敗する。↓汚いので書き直す
		while value.as_io().is_ok() {
			let (io, loc) = value.as_io().unwrap();
			value = RefCell::<dyn Io>::borrow_mut(&io).perform(&loc, imports) ?;
		}

		Ok(value)

} else {
		Err(error(ErrorType::DirectiveArgNotFound, stmt_loc.clone()))
	}
}

fn set_mute_solo(mute_solo: MuteSolo, tracks: &Vec<String>, pctx: &mut PlayerContext) {
	(*pctx).mute_solo = mute_solo;
	(*pctx).mute_solo_tracks.clear();
	tracks.iter().for_each(|t| {
		(*pctx).mute_solo_tracks.insert(t.clone());
	});
}
