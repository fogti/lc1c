use crate::statement::{Statement, StatementInvoc as StInvoc};

/// This structure describes what part of the tuple is kept
#[derive(Clone, PartialEq, Eq)]
pub enum FlatOptimizerRpl {
    None,
    First,
    Second,
    Both,
    Custom(Vec<Statement>),
}

impl FlatOptimizerRpl {
    #[inline]
    pub fn with_n(n: u8) -> FlatOptimizerRpl {
        match n {
            0 => FlatOptimizerRpl::None,
            1 => FlatOptimizerRpl::First,
            2 => FlatOptimizerRpl::Second,
            3 => FlatOptimizerRpl::Both,
            _ => panic!("FlatOptimizerRpl::with_n: got illegal $n = {}", n),
        }
    }
}

pub fn optimize_flat(
    stmts: &mut Vec<Statement>,
    drvf: fn(invoc: (&StInvoc, &StInvoc)) -> FlatOptimizerRpl,
) {
    loop {
        let olen = stmts.len();
        if olen <= 1 {
            break;
        }
        let old_stmts = std::mem::replace(stmts, Vec::with_capacity(olen));
        let mut it = old_stmts[..].windows(2).peekable();

        while let Some([i, inxt]) = it.next() {
            if !i.optimizable || !inxt.optimizable {
                stmts.push(i.clone());
                continue;
            }

            let invoc = (&i.invoc, &inxt.invoc);
            let mut drv_ret = flatdrv::pre(invoc);
            use FlatOptimizerRpl::{Both, Custom, First, Second};
            if drv_ret == FlatOptimizerRpl::Both {
                drv_ret = drvf(invoc);
            }
            let nn = if drv_ret == Both {
                Some(inxt)
            } else {
                it.next().map(|x| &x[1])
            };

            match drv_ret {
                First | Both => stmts.push(i.clone()),
                Second => stmts.push(inxt.clone()),
                Custom(y) => stmts.extend(y),
                _ => {}
            }
            if it.peek().is_none() {
                if let Some(nxt) = nn {
                    stmts.push(nxt.clone());
                }
            }
        }

        if stmts.len() == olen {
            break;
        }
    }
    stmts.shrink_to_fit();
}

pub mod flatdrv {
    use super::*;
    pub(crate) fn pre(invoc: (&StInvoc, &StInvoc)) -> FlatOptimizerRpl {
        use crate::statement::StatementInvocBase::*;
        FlatOptimizerRpl::with_n(match invoc {
            // opposite ops
            (&NOT, &NOT) | (&ADD, &SUB) | (&SUB, &ADD) => 0,
            (&RRA(ref r), &RLA(ref l)) | (&RLA(ref l), &RRA(ref r)) if r == l => 0,

            // direct overwrite
            (&LDA(_), &LDA(_)) | (&LDB(_), &LDB(_)) => 2,

            // no-ops
            (&AND, &AND)
            | (&MAB, &MAB)
            | (&JMP(_), &JMP(_))
            | (&JMP(_), &JPS(_))
            | (&JMP(_), &JPO(_))
            | (&JPS(_), &JPS(_))
            | (&JPO(_), &JPO(_))
            | (&RET, &CAL(_))
            | (&RET, &RET)
            | (&RET, &JMP(_))
            | (&HLT, &JMP(_))
            | (&HLT, &RET)
            | (&HLT, &HLT) => 1,

            _ => 3,
        })
    }

    /// this is a dummy optimizer
    pub fn generic(_: (&StInvoc, &StInvoc)) -> FlatOptimizerRpl {
        FlatOptimizerRpl::Both
    }
}
