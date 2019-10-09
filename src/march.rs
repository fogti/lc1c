use crate::{
    optimize,
    statement::{Statement, StatementInvoc as StInvoc, StatementInvocBase},
};

pub trait MArch {
    fn encode(&self, invoc: &StInvoc) -> Option<u16>;
    fn optimize_flat(&self, stmts: &mut Vec<Statement>);
}

pub struct LC1D;

impl MArch for LC1D {
    fn encode(&self, invoc: &StInvoc) -> Option<u16> {
        use StatementInvocBase::*;
        Some(match invoc {
            LDA(_) => 0x0,
            LDB(_) => 0x1,
            MOV(_) => 0x2,
            MAB => 0x3,
            ADD => 0x4,
            SUB => 0x5,
            AND => 0x6,
            NOT => 0x7,
            JMP(_) => 0x8,
            JPS(_) => 0x9,
            JPO(_) => 0xa,
            CAL(_) => 0xb,
            RET => 0xc,
            RRA(_) => 0xd,
            RLA(_) => 0xe,
            HLT => 0xf,
            DEF(_) => 0x0,
            NOP => 0xf,
            _ => return None,
        })
    }

    fn optimize_flat(&self, stmts: &mut Vec<Statement>) {
        fn flatdrv_lc1(invoc: (&StInvoc, &StInvoc)) -> optimize::FlatOptimizerRpl {
            use StatementInvocBase::*;
            optimize::FlatOptimizerRpl::with_n(match invoc {
                // direct overwrite
                (&NOT, &LDA(_))
                | (&ADD, &LDA(_))
                | (&SUB, &LDA(_))
                | (&MAB, &LDB(_))
                | (&LDB(_), &MAB) => 2,

                _ => 3,
            })
        }
        optimize::optimize_flat(stmts, flatdrv_lc1);
    }
}
