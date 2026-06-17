use simcli::{CliNoMut, OptTyp, OptVal, WildCardExpansion};
use std::error::Error;

#[cfg(test)]
fn test_cli(cli: &CliNoMut) -> Result<(), Box<dyn Error>> {
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
            .alias("-help")?
            .description("Print help")
            .opt("v", OptTyp::None)?
            .alias("-version")?
            .description("Print version")
            .opt(if cfg!(windows) { "R" } else { "H" }, OptTyp::None)?
            .alias("-charm")?
            .description("Nice feature")
            .opt("-long", OptTyp::Str)?
            .description("Long definition")
            .opt("-pat", OptTyp::ArrStr)?
            .description("Long multiple patters")
            .use_oper()
            .oper_description(
                r#"Where operations are:
add - add,
delete - delete,
commit - commit.
And options are:"#,
            )?;
        let _ = cli.process_wildcard(WildCardExpansion::All);
        let d_o = cli.get_opt("D");
        if let Ok(Some(OptVal::Arr(d_o))) = d_o {
            for (i, d) in d_o.into_iter().enumerate() {
                eprintln!("opt[{i}] {}={}", d.0, d.1);
            }
        } else {
            eprintln!("no def found")
        }
        if let Ok(Some(OptVal::Str(value))) = cli.get_opt("-long") {
            println!("long - {value}")
        }
        if cli.get_opt("k").ok().is_some() {
            println!("k is defined")
        }
        if cli.get_opt("v").ok().is_some() {
            println!("version - {}", simcli::get_version())
        }
        let _ = cli.opt("X", OptTyp::Str).inspect_err(|e| eprintln!("{e}"));
        for arg in &cli.args() {
            println!("arg - {arg}")
        }
        if let Some(oper) = cli.get_oper() {
            println!("operation - {oper} is rquested")
        } else {
            eprintln!("no operation specified")
        }
        if let Some(errors) = cli.get_errors() {
            eprintln!("Unknown options - {errors:?}")
        }
    }
    if let Ok(Some(OptVal::Str(name))) = cli.get_opt("n") {
        for _ in 0..if let Ok(Some(OptVal::Num(count))) = cli.get_opt("c")
            && count > 0
        {
            dbg!(count)
        } else {
            1
        } {
            eprintln!("Hello {name}!");
        }
    } else if cli.get_opt("h").ok().is_some() || cli.get_errors().is_some() {
        eprintln!("{}", cli.get_description().unwrap())
    }
    if let Ok(Some(OptVal::ArrStr(patterns))) = cli.get_opt("-pat") {
        eprintln!("patterns: {patterns:?}")
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = CliNoMut::new();
    cli.description("For testing CLI module");
    #[cfg(test)]
    test_cli(&cli)
}
