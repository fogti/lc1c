use lc1c::prelude::*;

fn main() {
    use clap::Arg;
    let matches = clap::App::new("lc1c")
        .version(clap::crate_version!())
        .author("Erik Zscheile <erik.zscheile@gmail.com>")
        .about("high-level LC1 asm compiler")
        .arg(
            Arg::with_name("INPUT")
                .help("sets the input file to use")
                .required(true)
                .index(1)
        )
        .arg(
            Arg::with_name("output")
                .short("o")
                .takes_value(true)
                .required(true)
                .help("specify a compilation output filename")
        )
        .arg(
            Arg::with_name("unix2dos")
                .short("U")
                .help("unix2dos mode -- insert carriage returns after each compiled line")
        )
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .help("be more verbose")
        )
        .arg(
            Arg::with_name("optimize")
                .short("O")
                .takes_value(true)
                .help("sets the optimization level; 0 = no optimization; 1 = normal optimization; D = deep optimization")
        )
        .get_matches();

    let input_file = matches.value_of("INPUT").unwrap();
    let output_file = matches.value_of("output").unwrap();
    let mut parsed = LC1CUnit::parse_from_file(input_file)
        .map_err(|()| std::process::exit(1))
        .unwrap();

    let march_lc1 = lc1c::march::LC1D;

    // 2. optimize
    match matches.value_of("optimize") {
        None | Some("0") => {}
        Some("D") => {}
        Some("1") => march_lc1.optimize_flat(&mut parsed.stmts),
        Some(x) => {
            panic!("LC1C: invalid '-O' (optimize) argument: {}", x);
        }
    }

    // 3. resolve Label's
    // 4. if -march=lc1: optimize IdConst's
    // 5. resolve IdConst's

    {
        let ofe = format!("file {}", output_file);
        let mut asm_out = lc1c::codegen::LC1Obj::new(output_file)
            .map_err(|x| lc1c::bailout_with_io_error(x, &ofe))
            .unwrap();
        asm_out
            .codegen(&parsed)
            .map_err(|x| lc1c::bailout_with_io_error(x, &ofe))
            .unwrap();
    }
}
