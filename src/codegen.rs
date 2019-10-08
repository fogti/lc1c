use crate::LC1CUnit;
use std::{fs, io::{self, Write}};

pub trait CodeGen {
    fn codegen(&mut self, u: &LC1CUnit) -> io::Result<()>;
}

/** LC1Asm is an output filter which outputs `_.LC1` code.
 * It strips dont-optimize markers from source code.
 **/
pub struct LC1Asm {
    dstf: fs::File,
}

impl LC1Asm {
    pub fn new(dstf_name: impl AsRef<std::path::Path>) -> io::Result<LC1Asm> {
        Ok(LC1Asm {
            dstf: fs::File::create(dstf_name)?,
        })
    }
}

impl CodeGen for LC1Asm {
    fn codegen(&mut self, u: &LC1CUnit) -> io::Result<()> {
        for i in u.stmts.iter() {
            if i.invoc.cmdcode() != None {
                write!(&mut self.dstf, "  ")?;
            }
            writeln!(&mut self.dstf, "{}", i.invoc)?;
        }
        Ok(())
    }
}
