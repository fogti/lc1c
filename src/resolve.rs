use crate::stmt::{Argument, CmdWithArg, Command, Statement};
use std::collections::HashMap;
use std::{convert::TryInto, mem::take};

#[derive(Clone, Debug)]
pub struct ResolveLabelError(pub CmdWithArg, pub String);

pub fn resolve_labels(
    stmts: &mut Vec<Statement>,
) -> Result<HashMap<String, u32>, ResolveLabelError> {
    // generate map of labels
    let mut labels = HashMap::new();
    let mut stcnt: u32 = 0;

    *stmts = take(stmts)
        .into_iter()
        .filter_map(|mut x| {
            if let Command::Label(ref mut l) = &mut x.cmd {
                labels.insert(take(l), stcnt);
                return None;
            }
            stcnt += 1;
            Some(x)
        })
        .collect();

    // resolve labels
    for x in stmts.iter_mut() {
        if let Command::Wa(cmd, ref mut arg) = &mut x.cmd {
            if let Argument::Label(ref mut l) = arg {
                if let Some(y) = labels.get(&*l) {
                    *arg =
                        Argument::Absolute((*y).try_into().expect("unable to convert label value"));
                } else {
                    return Err(ResolveLabelError(*cmd, take(l)));
                }
            }
        }
    }

    Ok(labels)
}

pub fn resolve_idconsts(stmts: &mut Vec<Statement>) {
    let mut iclbls = bimap::BiHashMap::<i32, usize>::new();
    let mut hicnt = stmts.len();

    for x in stmts.iter_mut() {
        match &mut x.cmd {
            Command::Def(ref mut arg) | Command::Wa(_, ref mut arg) => {
                if let Argument::IdConst(ic) = arg {
                    if iclbls.insert_no_overwrite(*ic, hicnt).is_ok() {
                        hicnt += 1;
                    }
                    let real_pos = *iclbls.get_by_left(ic).unwrap();
                    *arg = Argument::Absolute(
                        real_pos.try_into().expect("unable to convert label value"),
                    );
                }
            }
            _ => {}
        }
    }

    let mut iclbls: Vec<_> = iclbls.iter().map(|(&idc, &pos)| (pos, idc)).collect();
    iclbls.sort();
    for (pos, idc) in iclbls.into_iter() {
        if pos != stmts.len() {
            eprintln!(
                "lc1c-resolve-idconsts: position mismatch (expected = {}, got = {})",
                pos,
                stmts.len()
            );
        }
        stmts.push(Statement {
            cmd: Command::Def(Argument::Absolute(idc)),
            loc: None,
            do_ignore: false,
        });
    }
}
