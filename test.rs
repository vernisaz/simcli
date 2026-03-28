use simcli::OptTyp;
use simcli::OptVal;
use simcli::CLI;
use std::error::Error;

#[cfg(test)]
fn test_cli(cli: &mut CLI) {
    match cli.opt("D", OptTyp::InStr) {
        Ok(ref mut cli) => {
            cli.description("A definition as name=value");
        }
        _ => (),
    }
    let _ = cli.opt("c", OptTyp::None).inspect_err(|e| eprintln!("{e}"));

    let d_o = cli.get_opt("D");
    if let Some(OptVal::Arr(d_o)) = d_o {
        for (i, d) in d_o.into_iter().enumerate() {
            eprintln!("opt[{i}] {}={}", d.0, d.1);
        }
    } else {
        eprintln!("no def found")
    }
    let _ = cli.opt("X", OptTyp::Str).inspect_err(|e| eprintln!("{e}"));
    for arg in cli.args() {
        println!("arg - {arg}")
    }
    if let Some(errors) = cli.get_errors() {
        eprintln!("Unknown options - {errors:?}")
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut cli = CLI::new();
    cli.description("For testing CLI module");
    #[cfg(test)]
    test_cli(&mut cli);
    Ok(())
}
