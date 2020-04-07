use crate::{stmt::*, utils::AddOffset};
use std::{collections::HashMap, mem::take};

#[derive(Clone, Debug)]
pub struct ResolveLabelError(pub CmdWithArg, pub String);

pub fn resolve_labels(
    stmts: &mut Vec<Statement>,
) -> Result<HashMap<String, usize>, ResolveLabelError> {
    // generate map of labels
    let mut labels = HashMap::new();
    let mut stcnt: usize = 0;

    *stmts = take(stmts)
        .into_iter()
        .filter_map(|mut x| {
            match &mut x.cmd {
                Command::Label(ref mut l) => {
                    labels.insert(take(l), stcnt);
                    return None;
                }
                Command::Wa(_, ref mut arg) => {
                    if let Argument::Relative(a) = *arg {
                        *arg = Argument::Absolute(
                            stcnt.add_offset(a).expect("invalid relative offset"),
                        );
                    }
                }
                _ => {}
            }
            stcnt += 1;
            Some(x)
        })
        .collect();

    // resolve labels
    for x in stmts.iter_mut() {
        match &mut x.cmd {
            Command::Wa(cmd, ref mut arg) => {
                if let Argument::Label(ref mut l) = arg {
                    if let Some(y) = labels.get(&*l) {
                        *arg = Argument::Absolute(*y);
                    } else {
                        return Err(ResolveLabelError(*cmd, take(l)));
                    }
                }
            }
            _ => {}
        }
    }

    Ok(labels)
}
