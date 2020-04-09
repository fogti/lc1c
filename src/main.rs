use lc1c::*;
use std::sync::Arc;

fn read_asm_from_file(fname: String) -> Result<Vec<stmt::Statement>, (Arc<str>, std::io::Error)> {
    let mut stmts = Vec::new();
    let fname: Arc<str> = fname.into();

    let file_content = std::fs::read_to_string(&*fname).map_err(|e| (fname.clone(), e))?;

    for (lineno, line) in file_content.lines().enumerate() {
        match stmt::Statement::parse(
            &line[..],
            stmt::Location {
                file: fname.clone(),
                line: lineno as u32,
            },
        ) {
            Ok(mut stmts_) => stmts.append(&mut stmts_),
            Err(e) => eprintln!("lc1c: error @ {}: {}", e.0, line),
        }
    }

    Ok(stmts)
}

fn print_module(module: &bb::Module) {
    println!("module ::");
    for (n, i) in module.bbs().iter().enumerate() {
        print!("BB({}): ", n);
        let labels: Vec<_> = module.labels_of_bb(n).collect();
        if !labels.is_empty() {
            let mut labels = labels.into_iter();
            print!("/* {}", labels.next().unwrap());
            for i in labels {
                print!(", {}", i);
            }
            print!(" */ ");
        }
        println!("{}", i);
    }
    println!("labels: {:?}", module.labels());
}

fn main() {
    for arg in std::env::args().skip(1) {
        println!("process file :: {}", arg);

        let stmts = match read_asm_from_file(arg) {
            Ok(s) => s,
            Err((f, e)) => {
                eprintln!("lc1c: error while reading from file {}: {}", f, e);
                continue;
            }
        };

        println!("stmts: {:?}\n... convert into basic blocks ...", stmts);

        let mut module = bb::Module::new(stmts).expect("module parsing failed");

        print_module(&module);

        println!("\n... run optimization ...");

        module.optimize();

        print_module(&module);
    }
}
