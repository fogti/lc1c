use std::fmt;

#[derive(Clone, Debug)]
pub enum Argument {
    Invalid,
    Absolute(i32),
    IdConst(i32),
    Label(String),
}

impl std::str::FromStr for Argument {
    type Err = std::num::ParseIntError;

    fn from_str(s: &str) -> Result<Argument, std::num::ParseIntError> {
        Ok(if s.is_empty() || s.contains(|i: char| i.is_whitespace()) {
            Argument::Invalid
        } else {
            match s.chars().next().unwrap() {
                '@' => Argument::Absolute(s[1..].parse()?),
                '$' => Argument::IdConst(s[1..].parse()?),
                '-' => Argument::Absolute(s.parse()?),
                x if x.is_digit(10) => Argument::Absolute(s.parse()?),
                _ => Argument::Label(s.to_string()),
            }
        })
    }
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
            A::Absolute(a) => {
                if f.alternate() {
                    write!(f, "{}", a)
                } else {
                    write!(f, "@{}", a)
                }
            }
            A::IdConst(a) => write!(f, "${}", a),
            A::Label(ref a) => write!(f, "{}", a),
        }
    }
}

#[derive(Clone, Debug)]
pub struct CommandParseError(pub String);

macro_rules! cmdxai {
    ($structn:ident, $(($cmdn:ident, $cmds:expr, $cmdv:expr)),+ $(,)?) => {
        #[derive(Clone, Copy, Debug)]
        pub enum $structn {
            $($cmdn = $cmdv),+
        }

        impl std::str::FromStr for $structn {
            type Err = CommandParseError;
            fn from_str(s: &str) -> Result<$structn, CommandParseError> {
                let s_ = s.to_ascii_uppercase();
                Ok(match s_.as_str() {
                    $($cmds => $structn::$cmdn),+,
                    _ => return Err(CommandParseError(s_)),
                })
            }
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
    (Lda, "LDA", 0x0),
    (Ldb, "LDB", 0x1),
    (Mov, "MOV", 0x2),
    (Jmp, "JMP", 0x8),
    (Jps, "JPS", 0x9),
    (Jpo, "JPO", 0xa),
    (Cal, "CAL", 0xb),
    (Rra, "RRA", 0xd),
    (Rla, "RLA", 0xe),
}

cmdxai! { CmdNoArg,
    (Mab, "MAB", 0x3),
    (Add, "ADD", 0x4),
    (Sub, "SUB", 0x5),
    (And, "AND", 0x6),
    (Not, "NOT", 0x7),
    (Ret, "RET", 0xc),
    (Hlt, "HLT", 0xf),
}

#[derive(Clone, Copy, Debug)]
pub enum CondJumpCond {
    Sign,
    Overflow,
}

impl From<CondJumpCond> for CmdWithArg {
    fn from(x: CondJumpCond) -> CmdWithArg {
        use CmdWithArg as Cwa;
        use CondJumpCond as Cjc;
        match x {
            Cjc::Sign => Cwa::Jps,
            Cjc::Overflow => Cwa::Jpo,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Command {
    Def(Argument),
    Label(String),
    Wa(CmdWithArg, Argument),
    Na(CmdNoArg),
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Command::Def(ref a) => write!(f, "\tDEF {:#}", a),
            Command::Label(ref a) => write!(f, "{}:", a),
            Command::Wa(ref c, ref a) => write!(f, "\t{} {}", c, a),
            Command::Na(ref c) => write!(f, "\t{}", c),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Location {
    pub file: std::sync::Arc<str>,
    // zero-based line number
    pub line: u32,
}

impl Default for Location {
    fn default() -> Self {
        Location {
            file: "<>".into(),
            line: 0,
        }
    }
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.file, u64::from(self.line) + 1)
    }
}

#[derive(Clone, Debug)]
pub struct Statement {
    pub loc: Location,
    pub cmd: Command,
    pub do_ignore: bool,
}

impl fmt::Display for Statement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.do_ignore {
            write!(f, "*")?;
        }
        write!(f, "{}\t; {}", self.cmd, self.loc)
    }
}

pub struct StatementParseError(pub Location);

impl Statement {
    pub fn parse(mut s: &str, loc: Location) -> Result<Vec<Self>, StatementParseError> {
        let mut ret = Vec::new();

        // discard comments
        if let Some(comma) = s.find(';') {
            s = &s[..comma];
        }
        s = s.trim();

        // parse labels
        while let Some(colon) = s.find(':') {
            let label = &s[..colon];
            s = s[colon + 1..].trim_start();
            ret.push(Statement {
                loc: loc.clone(),
                cmd: Command::Label(label.to_string()),
                do_ignore: false,
            });
        }

        if s.is_empty() {
            return Ok(ret);
        }

        let mut do_ignore = false;
        let xths: Vec<_> = s.split_whitespace().collect();
        let (mut x_cmd, x_args): (&str, &str) = if let [x_cmd, x_args] = &xths[..] {
            (x_cmd, x_args)
        } else if let [x_cmd] = &xths[..] {
            (x_cmd, "")
        } else {
            return Err(StatementParseError(loc));
        };
        std::mem::drop(xths);

        let x_args = if x_args.is_empty() {
            None
        } else {
            Some(
                x_args
                    .parse()
                    .map_err(|_| StatementParseError(loc.clone()))?,
            )
        };
        if x_cmd.starts_with('*') {
            do_ignore = true;
            x_cmd = &x_cmd[1..];
        }

        let cmd = if let Some(args) = x_args {
            if x_cmd.eq_ignore_ascii_case("def") {
                Command::Def(args)
            } else {
                Command::Wa(
                    x_cmd
                        .parse()
                        .map_err(|_| StatementParseError(loc.clone()))?,
                    args,
                )
            }
        } else {
            Command::Na(
                x_cmd
                    .parse()
                    .map_err(|_| StatementParseError(loc.clone()))?,
            )
        };
        ret.push(Statement {
            loc,
            cmd,
            do_ignore,
        });

        Ok(ret)
    }
}
