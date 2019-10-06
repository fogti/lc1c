use std::{collections::HashMap, fmt, str};

pub trait ToStaticStr: Copy {
    fn to_static_str(self) -> &'static str;
    fn _display(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_static_str())
    }
}

macro_rules! impl_display_for_static_str {
    ($struct:ident) => {
        impl fmt::Display for $struct {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                ToStaticStr::_display(self, f)
            }
        }
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VirtCommand {
    None,
    Def,
    Label,
}

impl ToStaticStr for VirtCommand {
    fn to_static_str(self) -> &'static str {
        match self {
            VirtCommand::None => "NONE",
            VirtCommand::Def => "DEF",
            VirtCommand::Label => "LABEL",
        }
    }
}

impl_display_for_static_str!(VirtCommand);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RealCommand {
    LDA = 0x0,
    LDB = 0x1,
    MOV = 0x2,
    MAB = 0x3,
    ADD = 0x4,
    SUB = 0x5,
    AND = 0x6,
    NOT = 0x7,

    JMP = 0x8,
    JPS = 0x9,
    JPO = 0xa,
    CAL = 0xb,
    RET = 0xc,
    RRA = 0xd,
    RLA = 0xe,
    HLT = 0xf,
}

impl_display_for_static_str!(RealCommand);

impl ToStaticStr for RealCommand {
    fn to_static_str(self) -> &'static str {
        use RealCommand::*;
        match self {
            LDA => "LDA",
            LDB => "LDB",
            MOV => "MOV",
            MAB => "MAB",
            ADD => "ADD",
            SUB => "SUB",
            AND => "AND",
            NOT => "NOT",

            JMP => "JMP",
            JPS => "JPS",
            JPO => "JPO",
            CAL => "CAL",
            RET => "RET",
            RRA => "RRA",
            RLA => "RLA",
            HLT => "HLT",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    V(VirtCommand),
    R(RealCommand),
}

impl ToStaticStr for Command {
    fn to_static_str(self) -> &'static str {
        match self {
            Command::V(x) => x.to_static_str(),
            Command::R(x) => x.to_static_str(),
        }
    }
}

impl_display_for_static_str!(Command);

impl Command {
    fn has_arg(self) -> bool {
        match self {
            Command::V(x) => match x {
                VirtCommand::None => false,
                VirtCommand::Def => true,
                VirtCommand::Label => true,
            },
            Command::R(x) => {
                use RealCommand::*;
                match x {
                    LDA | LDB | MOV => true,
                    JMP | JPS | JPO | CAL | RRA | RLA => true,
                    _ => false,
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AddrType {
    Invalid,
    None,
    Absolute,
    Relative,
    IdConst,
    Label,
}

impl ToStaticStr for AddrType {
    fn to_static_str(self) -> &'static str {
        match self {
            AddrType::Invalid => "invalid",
            AddrType::None => "none",
            AddrType::Absolute => "absolute",
            AddrType::Relative => "relative",
            AddrType::IdConst => "ind.const",
            AddrType::Label => "label",
        }
    }
}

impl_display_for_static_str!(AddrType);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Argument {
    Absolute(u16),
    Relative(i16),
    IdConst(u16),
    Label(String),
}

impl fmt::Display for Argument {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Argument::*;
        match self {
            Absolute(x) => write!(f, "@{}", x),
            Relative(x) => write!(f, ".{}", x),
            IdConst(x) => write!(f, "${}", x),
            Label(ref x) => write!(f, "{}", x),
        }
    }
}

pub struct ParseArgumentError(Option<std::num::ParseIntError>);

impl str::FromStr for Argument {
    type Err = ParseArgumentError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use Argument::*;
        // match on first char
        Ok(match s.chars().next().unwrap() {
            '@' => Absolute(s.split_at(1).1.parse()?),
            '.' => Relative(s.split_at(1).1.parse()?),
            '$' => IdConst(s.split_at(1).1.parse()?),
            _ => Label(s.to_string()),
        })
    }
}

impl From<std::num::ParseIntError> for ParseArgumentError {
    fn from(error: std::num::ParseIntError) -> Self {
        ParseArgumentError(Some(error))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StatementInvocBase<T> {
    LDA(T),
    LDB(T),
    MOV(T),
    MAB,
    ADD,
    SUB,
    AND,
    NOT,

    JMP(T),
    JPS(T),
    JPO(T),
    CAL(T),
    RET,
    RRA(T),
    RLA(T),
    HLT,

    DEF(u16),
    Label(String),
}

pub type StatementInvoc = StatementInvocBase<Argument>;

pub struct Statement {
    pub invoc: StatementInvoc,
    pub optimizable: bool,
}

type Labels = HashMap<String, usize>;

impl StatementInvoc {
    pub fn arg(&self) -> Option<&Argument> {
        use StatementInvocBase::*;
        match self {
            LDA(ref x) | LDB(ref x) | MOV(ref x) | JMP(ref x) | JPS(ref x) | JPO(ref x)
            | CAL(ref x) | RRA(ref x) | RLA(ref x) => Some(x),
            _ => None,
        }
    }

    pub fn cmd2str(&self) -> &'static str {
        use StatementInvocBase::*;
        match self {
            LDA(_) => "LDA",
            LDB(_) => "LDB",
            MOV(_) => "MOV",
            MAB => "MAB",
            ADD => "ADD",
            SUB => "SUB",
            AND => "AND",
            NOT => "NOT",

            JMP(_) => "JMP",
            JPS(_) => "JPS",
            JPO(_) => "JPO",
            CAL(_) => "CAL",
            RET => "RET",
            RRA(_) => "RRA",
            RLA(_) => "RLA",
            HLT => "HLT",

            DEF(_) => "DEF",
            Label(_) => "LABEL",
        }
    }

    pub fn into_statement(self, optimizable: bool) -> Statement {
        Statement {
            invoc: self,
            optimizable,
        }
    }
}

impl<T> StatementInvocBase<T> {
    fn map_or_fail<U, E, Fn: FnOnce(T) -> Result<U, E>>(self, f: Fn) -> Result<StatementInvocBase<U>, E> {
        use StatementInvocBase::*;
        Ok(match self {
            LDA(x) => LDA(f(x)?),
            LDB(x) => LDB(f(x)?),
            MOV(x) => MOV(f(x)?),
            MAB => MAB,
            ADD => ADD,
            SUB => SUB,
            AND => AND,
            NOT => NOT,

            JMP(x) => JMP(f(x)?),
            JPS(x) => JPS(f(x)?),
            JPO(x) => JPO(f(x)?),
            CAL(x) => CAL(f(x)?),
            RET => RET,
            RRA(x) => RRA(f(x)?),
            RLA(x) => RLA(f(x)?),
            HLT => HLT,

            DEF(x) => DEF(x),
            Label(x) => Label(x),
        })
    }
}

impl fmt::Display for StatementInvoc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StatementInvoc::DEF(x) => write!(f, "DEF {}", x),
            StatementInvoc::Label(ref x) => write!(f, "{}:", x),
            _ => {
                write!(f, "{}", self.cmd2str())?;
                if let Some(x) = self.arg() {
                    write!(f, " {}", x)
                } else {
                    Ok(())
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseStatementError {
    TooShort,
    UnexpectedArgument,
    ArgumentNotFound,
    InvalidArgument,
    TooManyTokens(usize),
    UnknownCommand,

    Integer(std::num::ParseIntError),
}

impl fmt::Display for ParseStatementError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use ParseStatementError::*;
        match self {
            TooShort => write!(f, "statement is invalid because it's too short"),
            UnexpectedArgument => write!(f, "expected no argument, found one"),
            ArgumentNotFound => write!(f, "expected one argument, found none"),
            InvalidArgument => write!(f, "argument is invalid"),
            TooManyTokens(n) => write!(f, "statement consists of too many (whitespace-separated) tokens (expected at most 2, got {})", n),
            UnknownCommand => write!(f, "got unknown command"),
            Integer(ref x) => write!(f, "parsing argument failed: {}", x),
        }
    }
}

impl std::error::Error for ParseStatementError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ParseStatementError::Integer(ref x) => Some(x),
            _ => None,
        }
    }
}

impl From<std::num::ParseIntError> for ParseStatementError {
    fn from(error: std::num::ParseIntError) -> Self {
        ParseStatementError::Integer(error)
    }
}

impl From<ParseArgumentError> for ParseStatementError {
    fn from(error: ParseArgumentError) -> Self {
        if let Some(ie) = error.0 {
            ParseStatementError::Integer(ie)
        } else {
            ParseStatementError::InvalidArgument
        }
    }
}

impl str::FromStr for StatementInvoc {
    type Err = ParseStatementError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        fn from_simple_str(s: &str) -> Option<StatementInvoc> {
            use StatementInvocBase::*;
            Some(match s {
                "MAB" => MAB,
                "ADD" => ADD,
                "SUB" => SUB,
                "AND" => AND,
                "NOT" => NOT,
                "RET" => RET,
                "HLT" => HLT,
                _ => return None,
            })
        }
        fn str_to_cmd_arg_tuple(s: &str) -> Result<(&str, Option<&str>), ParseStatementError> {
            let parts: Vec<&str> = s.split_whitespace().collect();
            match parts.len() {
                0 => Err(TooShort),
                1 => Ok((parts[0], None)),
                2 => Ok((parts[0], Some(parts[1]))),
                n => Err(TooManyTokens(n)),
            }
        }

        if s.len() < 2 {
            return Err(ParseStatementError::TooShort);
        } else if s.ends_with(':') {
            // got label
            return Ok(StatementInvoc::Label(s.split_at(s.len() - 1).0.to_string()));
        }

        use ParseStatementError::*;
        let (cmd, arg) = str_to_cmd_arg_tuple(s)?;
        let cmd = cmd.to_uppercase();
        let cmd = cmd.as_str();

        if let Some(x) = from_simple_str(cmd) {
            if arg.is_some() {
                Err(UnexpectedArgument)
            } else {
                Ok(x)
            }
        } else if let Some(arg) = arg {
            Ok(match cmd {
                "DEF" => StatementInvocBase::DEF(arg.parse::<u16>()?),
                "LDA" => StatementInvocBase::LDA(()),
                "LDB" => StatementInvocBase::LDB(()),
                "MOV" => StatementInvocBase::MOV(()),
                "JMP" => StatementInvocBase::JMP(()),
                "JPS" => StatementInvocBase::JPS(()),
                "JPO" => StatementInvocBase::JPO(()),
                "CAL" => StatementInvocBase::CAL(()),
                "RRA" => StatementInvocBase::RRA(()),
                "RLA" => StatementInvocBase::RLA(()),
                _ => return Err(UnknownCommand),
            }.map_or_fail(|_| arg.parse::<Argument>())?)
        } else {
            Err(ArgumentNotFound)
        }
    }
}
