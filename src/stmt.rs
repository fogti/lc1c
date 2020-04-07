use std::fmt;

#[derive(Clone, Debug)]
pub enum Argument {
    Invalid,
    Absolute(usize),
    Relative(isize),
    IdConst(i32),
    Label(String),
}

impl std::default::Default for Argument {
    fn default() -> Argument {
        Argument::Invalid
    }
}

impl fmt::Display for Argument {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Argument as A;
        match self {
            A::Invalid => write!(f, "(invalid)"),
            A::Absolute(a) => write!(f, "@{}", a),
            A::Relative(a) => write!(f, ".{}", a),
            A::IdConst(a) => write!(f, "${}", a),
            A::Label(ref a) => write!(f, "{}", a),
        }
    }
}

macro_rules! cmdxai {
    ($structn:ident, $(($cmdn:ident, $cmds:expr, $cmdv:expr)),+ $(,)?) => {
        #[derive(Clone, Copy, Debug)]
        pub enum $structn {
            $($cmdn = $cmdv),+
        }

        impl fmt::Display for $structn {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let cs = match *self {
                    $($structn::$cmdn => $cmds),+
                };
                write!(f, "{}", cs)
            }
        }
    }
}

cmdxai! { CmdWithArg,
    (Lda, "LDA", 0x10),
    (Ldb, "LDB", 0x11),
    (Mov, "MOV", 0x12),
    (Jmp, "JMP", 0x18),
    (Jps, "JPS", 0x19),
    (Jpo, "JPO", 0x1a),
    (Cal, "CAL", 0x1b),
    (Rra, "RRA", 0x1d),
    (Rla, "RLA", 0x1e),
}

cmdxai! { CmdNoArg,
    (Def, "DEF", 0x01),
    (Mab, "MAB", 0x13),
    (Add, "ADD", 0x14),
    (Sub, "SUB", 0x15),
    (And, "AND", 0x16),
    (Not, "NOT", 0x17),
    (Ret, "RET", 0x1c),
    (Hlt, "HLT", 0x1f),
}

#[derive(Clone, Debug)]
pub enum Command {
    None,
    Def(Argument),
    Label(String),
    Wa(CmdWithArg, Argument),
    Na(CmdNoArg),
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Command::None => write!(f, "-UKN-"),
            Command::Def(ref a) => write!(f, "DEF {}", a),
            Command::Label(ref a) => write!(f, "LABEL {}", a),
            Command::Wa(ref c, ref a) => write!(f, "{} {}", c, a),
            Command::Na(ref c) => write!(f, "{}", c),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Statement {
    pub cmd: Command,
    pub do_ignore: bool,
}
