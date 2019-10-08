#![feature(never_type)]

pub mod codegen;
pub use codegen::CodeGen;
pub mod statement;
pub use statement::Statement;

#[derive(Clone, Debug)]
pub struct LC1CUnit {
    stmts: Vec<Statement>,
}

impl LC1CUnit {
    pub fn parse(s: &str, src_name: &str) -> Result<LC1CUnit, ()> {
        let mut is_success = true;
        let mut ret = LC1CUnit { stmts: vec![] };
        for (n, i) in s.lines().enumerate() {
            let orig_i = i;
            // strip comments
            let i = i.split(';').next().unwrap();
            let i = i.trim_start();
            if i.is_empty() {
                continue;
            }
            let mut istk = vec![i];
            while let Some(mut i) = istk.pop() {
                // a statement starting with '-' is marked as non-optimizable
                let optimizable = if i.starts_with('-') {
                    i = i.split_at(1).1;
                    false
                } else {
                    true
                };
                let i = i.trim();
                match i.parse::<statement::StatementInvoc>() {
                    Err(x) if x == statement::ParseStatementError::InlineLabel => {
                        let (a, b) = i.split_at(i.find(':').unwrap() + 1);
                        istk.push(a);
                        istk.push(b);
                    }
                    Err(x) => {
                        use colored::Colorize;
                        eprintln!(
                            "{}: {}",
                            "LC1C ERROR".bright_red().bold(),
                            format!("{}", x).bold()
                        );
                        eprintln!("    ╭─> {}:{}", src_name, n);
                        eprintln!("    │");
                        eprintln!("{} ┴ {}", format!("{: >3}", n).blue().bold(), orig_i);
                        eprintln!("");
                        is_success = false;
                    }
                    Ok(x) => {
                        ret.stmts.push(x.into_statement(optimizable));
                    }
                }
            }
        }
        if is_success {
            Ok(ret)
        } else {
            Err(())
        }
    }

    pub fn parse_from_file(file_name: &str) -> Result<LC1CUnit, ()> {
        let fh = readfilez::read_from_file(std::fs::File::open(file_name)).map_err(|x| {
            print_io_error(x, &format!("file {}", file_name));
        })?;
        let fh = std::str::from_utf8(&fh).map_err(|x| {
            use colored::Colorize;
            eprintln!(
                "{}: {}",
                "LC1C ERROR".bright_red().bold(),
                format!("file '{}' contains non-utf8 data", file_name).bold()
            );
            eprintln!("    ──> {}", x);
        })?;
        LC1CUnit::parse(fh, file_name)
    }
}

fn print_io_error(err: std::io::Error, origin: &str) {
    use colored::Colorize;
    eprintln!(
        "{}: {}",
        "LC1C IO ERROR".bright_red().bold(),
        format!("{}: {}", origin, err).bold()
    );
}

pub fn bailout_with_io_error(err: std::io::Error, origin: &str) -> ! {
    print_io_error(err, origin);
    std::process::exit(1);
}
