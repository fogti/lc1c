use std::{collections::HashMap, fmt, str};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Argument {
    Absolute(u16),
    Relative(i16),
    IdConst(u16),
    Label(String),
}

impl Argument {
    pub fn get_type(&self) -> (char, &'static str) {
        use Argument::*;
        match self {
            Absolute(_) => ('@', "absolute"),
            Relative(_) => ('.', "relative"),
            IdConst(_) => ('$', "ind.const"),
            Label(_) => (':', "label"),
        }
    }
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

pub trait StatementInvocBackend {
    type DefCode;
    type Label;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StatementInvocBase<T: StatementInvocBackend> {
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

    DEF(T::DefCode),
    Label(T::Label),
}

impl StatementInvocBackend for Argument {
    type DefCode = u16;
    type Label = String;
}

pub type StatementInvoc = StatementInvocBase<Argument>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Statement {
    pub invoc: StatementInvoc,
    pub optimizable: bool,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Command {
    code: Option<u8>,
    mnemonic: &'static str,
    is_real: bool,
    has_arg: bool,
}

type Labels = HashMap<String, usize>;

impl StatementInvoc {
    pub fn into_statement(self, optimizable: bool) -> Statement {
        Statement {
            invoc: self,
            optimizable,
        }
    }
}

static _CMD_DATA: &'static [Command] = &[
    Command {
        code: Some(0x0),
        mnemonic: "LDA",
        is_real: true,
        has_arg: true,
    },
    Command {
        code: Some(0x1),
        mnemonic: "LDB",
        is_real: true,
        has_arg: true,
    },
    Command {
        code: Some(0x2),
        mnemonic: "MOV",
        is_real: true,
        has_arg: true,
    },
    Command {
        code: Some(0x3),
        mnemonic: "MAB",
        is_real: true,
        has_arg: false,
    },
    Command {
        code: Some(0x4),
        mnemonic: "ADD",
        is_real: true,
        has_arg: false,
    },
    Command {
        code: Some(0x5),
        mnemonic: "SUB",
        is_real: true,
        has_arg: false,
    },
    Command {
        code: Some(0x6),
        mnemonic: "AND",
        is_real: true,
        has_arg: false,
    },
    Command {
        code: Some(0x7),
        mnemonic: "NOT",
        is_real: true,
        has_arg: false,
    },
    Command {
        code: Some(0x8),
        mnemonic: "JMP",
        is_real: true,
        has_arg: true,
    },
    Command {
        code: Some(0x9),
        mnemonic: "JPS",
        is_real: true,
        has_arg: true,
    },
    Command {
        code: Some(0xa),
        mnemonic: "JPO",
        is_real: true,
        has_arg: true,
    },
    Command {
        code: Some(0xb),
        mnemonic: "CAL",
        is_real: true,
        has_arg: true,
    },
    Command {
        code: Some(0xc),
        mnemonic: "RET",
        is_real: true,
        has_arg: false,
    },
    Command {
        code: Some(0xd),
        mnemonic: "RRA",
        is_real: true,
        has_arg: true,
    },
    Command {
        code: Some(0xe),
        mnemonic: "RLA",
        is_real: true,
        has_arg: true,
    },
    Command {
        code: Some(0xf),
        mnemonic: "HLT",
        is_real: true,
        has_arg: false,
    },
    Command {
        code: Some(0x0),
        mnemonic: "DEF",
        is_real: false,
        has_arg: true,
    },
    Command {
        code: None,
        mnemonic: "LABEL",
        is_real: false,
        has_arg: true,
    },
    Command {
        code: None,
        mnemonic: "-TERMINATOR-",
        is_real: false,
        has_arg: false,
    },
];

impl<T: StatementInvocBackend> StatementInvocBase<T> {
    fn map_or_fail<U, E, Fn, DFn, LFn>(
        self,
        f: Fn,
        df: DFn,
        lf: LFn,
    ) -> Result<StatementInvocBase<U>, E>
    where
        U: StatementInvocBackend,
        Fn: FnOnce(T) -> Result<U, E>,
        DFn: FnOnce(T::DefCode) -> Result<U::DefCode, E>,
        LFn: FnOnce(T::Label) -> Result<U::Label, E>,
    {
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

            DEF(x) => DEF(df(x)?),
            Label(x) => Label(lf(x)?),
        })
    }

    pub fn arg(&self) -> Option<&T> {
        use StatementInvocBase::*;
        match self {
            LDA(ref x) | LDB(ref x) | MOV(ref x) | JMP(ref x) | JPS(ref x) | JPO(ref x)
            | CAL(ref x) | RRA(ref x) | RLA(ref x) => Some(x),
            _ => None,
        }
    }

    /// get_cmd -> (cmdcode, cmd2str, is_real, has_arg)
    pub fn get_cmd(&self) -> Command {
        use StatementInvocBase::*;
        let ret = match self {
            LDA(_) => 0x00,
            LDB(_) => 0x01,
            MOV(_) => 0x02,
            MAB => 0x03,
            ADD => 0x04,
            SUB => 0x05,
            AND => 0x06,
            NOT => 0x07,
            JMP(_) => 0x08,
            JPS(_) => 0x09,
            JPO(_) => 0x0a,
            CAL(_) => 0x0b,
            RET => 0x0c,
            RRA(_) => 0x0d,
            RLA(_) => 0x0e,
            HLT => 0x0f,
            DEF(_) => 0x10,
            Label(_) => 0x11,
        };
        _CMD_DATA[ret]
    }

    pub fn cmdcode(&self) -> Option<u8> {
        self.get_cmd().code
    }

    pub fn cmd2str(&self) -> &'static str {
        self.get_cmd().mnemonic
    }

    pub fn is_cmd_real(&self) -> bool {
        self.get_cmd().is_real
    }

    pub fn has_arg(self) -> bool {
        self.get_cmd().has_arg
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

struct ParserWoArg;

impl StatementInvocBackend for ParserWoArg {
    type DefCode = ParserWoArg;
    type Label = String;
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
                "DEF" => StatementInvocBase::DEF(ParserWoArg),
                "LDA" => StatementInvocBase::LDA(ParserWoArg),
                "LDB" => StatementInvocBase::LDB(ParserWoArg),
                "MOV" => StatementInvocBase::MOV(ParserWoArg),
                "JMP" => StatementInvocBase::JMP(ParserWoArg),
                "JPS" => StatementInvocBase::JPS(ParserWoArg),
                "JPO" => StatementInvocBase::JPO(ParserWoArg),
                "CAL" => StatementInvocBase::CAL(ParserWoArg),
                "RRA" => StatementInvocBase::RRA(ParserWoArg),
                "RLA" => StatementInvocBase::RLA(ParserWoArg),
                _ => return Err(UnknownCommand),
            }
            .map_or_fail(
                |_| arg.parse::<Argument>(),
                |_| arg.parse::<u16>().map_err(|x| x.into()),
                |x| Ok(x),
            )?)
        } else {
            Err(ArgumentNotFound)
        }
    }
}