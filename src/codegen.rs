use crate::CompileUnit;
use std::io::{self, Write};

pub trait CodeGen {
    fn codegen(&self, u: &CompileUnit, f: &mut dyn io::Write) -> io::Result<()>;

    fn new() -> Self
    where
        Self: Default,
    {
        std::default::Default::default()
    }
}

/** LC1Asm is an output filter which outputs `_.LC1` code.
 * It strips dont-optimize markers from source code.
 **/
#[derive(Clone, Copy, Default)]
pub struct LC1Asm;

impl CodeGen for LC1Asm {
    fn codegen(&self, u: &CompileUnit, f: &mut dyn io::Write) -> io::Result<()> {
        for i in u.stmts.iter() {
            if let crate::statement::StatementInvocBase::Label(_) = &i.invoc {
                // do nothing
            } else {
                write!(f, "  ")?;
            }
            writeln!(f, "{}", i.invoc)?;
        }
        Ok(())
    }
}

/// LC1Obj is an output filter which outputs `_.LC1O` code.
#[derive(Clone, Copy, Default)]
pub struct LC1Obj;

impl CodeGen for LC1Obj {
    fn codegen(&self, u: &CompileUnit, f: &mut dyn io::Write) -> io::Result<()> {
        for (n, i) in u
            .stmts
            .iter()
            .filter(|i| {
                if let crate::statement::StatementInvocBase::Label(_) = &i.invoc {
                    false
                } else {
                    true
                }
            })
            .enumerate()
        {
            writeln!(f, "{} {}", n, i.invoc)?;
        }
        Ok(())
    }
}
