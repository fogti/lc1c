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
            let mut i = i.trim_start();
            if i.is_empty() {
                continue;
            }
            // a statement starting with '-' is marked as non-optimizable
            let optimizable = if i.starts_with('-') {
                i = i.split_at(1).1;
                false
            } else {
                true
            };
            let i = i.trim();
            match i.parse::<statement::StatementInvoc>() {
                Err(x) => {
                    use colored::Colorize;
                    eprintln!(
                        "{}: {}",
                        "LC1C ERROR".bright_red().bold(),
                        format!("{}", x).bold()
                    );
                    eprintln!("    ╭─> {}:{}", src_name, n);
                    eprintln!("    │");
                    eprintln!("{: >3} ┴ {}", n, orig_i);
                    eprintln!("");
                    is_success = false;
                }
                Ok(x) => {
                    ret.stmts.push(x.into_statement(optimizable));
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
            use colored::Colorize;
            eprintln!(
                "{}: {}",
                "LC1C IO ERROR".bright_red().bold(),
                format!("file {}: {}", file_name, x).bold()
            );
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
