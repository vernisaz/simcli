use simcli::CLI;
use simcli::OptTyp;
use simcli::OptVal;
use std::error::Error;

#[cfg(test)]
fn test_cli(cli: &mut CLI) -> Result<(), Box<dyn Error>> {
    {
        match cli.opt("D", OptTyp::InStr) {
            Ok(ref mut cli) => {
                cli.description("A definition as name=value");
            }
            _ => (),
        }
        let _ = cli.opt("k", OptTyp::None).inspect_err(|e| eprintln!("{e}"));
        cli.opt("n", OptTyp::Str)?
            .description("Name of the person to greet")
            .opt("c", OptTyp::Num)?
            .description("Number of times to greet [default: 1]")
            .opt("h", OptTyp::None)?
            .description("Print help")
            .opt("v", OptTyp::None)?
            .description("Print version");

        let d_o = cli.get_opt("D");
        if let Some(OptVal::Arr(d_o)) = d_o {
            for (i, d) in d_o.into_iter().enumerate() {
                eprintln!("opt[{i}] {}={}", d.0, d.1);
            }
        } else {
            eprintln!("no def  found")
        }
        let _ = cli.opt("X", OptTyp::Str).inspect_err(|e| eprintln!("{e}"));
        for arg in cli.args() {
            println!("arg - {arg}")
        }
        if let Some(errors) = cli.get_errors() {
            eprintln!("Unknown options - {errors:?}")
        }
    }
    if let Some(OptVal::Str(name)) = cli.get_opt("n") {
        let name = name.clone();
        for _ in 0..if let Some(OptVal::Num(count)) = cli.get_opt("c")
            && *count > 0
        {
            *count
        } else {
            1
        } {
            println!("Hello {name}!");
        }
    } else {
        eprintln!("{}", cli.get_description().unwrap())
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut cli = CLI::new();
    cli.description("For testing CLI module");
    #[cfg(test)]
    test_cli(&mut cli)
}
