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
