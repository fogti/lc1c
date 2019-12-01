use crate::CompileUnit;
use std::{
    fs,
    io::{self, Write},
};

pub trait CodeGen {
    fn codegen(&mut self, u: &CompileUnit) -> io::Result<()>;
}

/** LC1Asm is an output filter which outputs `_.LC1` code.
 * It strips dont-optimize markers from source code.
 **/
pub struct LC1Asm {
    dstf: fs::File,
}

impl LC1Asm {
    pub fn new(dstf_name: impl AsRef<std::path::Path>) -> io::Result<Self> {
        Ok(Self {
            dstf: fs::File::create(dstf_name)?,
        })
    }
}

impl CodeGen for LC1Asm {
    fn codegen(&mut self, u: &CompileUnit) -> io::Result<()> {
        for i in u.stmts.iter() {
            if let &crate::statement::StatementInvocBase::Label(_) = &i.invoc {
                // do nothing
            } else {
                write!(&mut self.dstf, "  ")?;
            }
            writeln!(&mut self.dstf, "{}", i.invoc)?;
        }
        Ok(())
    }
}

/// LC1Obj is an output filter which outputs `_.LC1O` code.
pub struct LC1Obj {
    dstf: fs::File,
}

impl LC1Obj {
    pub fn new(dstf_name: impl AsRef<std::path::Path>) -> io::Result<Self> {
        Ok(Self {
            dstf: fs::File::create(dstf_name)?,
        })
    }
}

impl CodeGen for LC1Obj {
    fn codegen(&mut self, u: &CompileUnit) -> io::Result<()> {
        for (n, i) in u.stmts.iter().enumerate() {
            if let &crate::statement::StatementInvocBase::Label(_) = &i.invoc {
                // do nothing
            } else {
                write!(&mut self.dstf, "{} ", n)?;
            }
            writeln!(&mut self.dstf, "{}", i.invoc)?;
        }
        Ok(())
    }
}