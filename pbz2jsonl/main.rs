use clap::{App, Arg};
use jsonpath_lib::select;
use pbzlib::PBZReader;
use serde_json::to_string_pretty;
use serde_json::Value;
use std::io;
use std::io::Write;

fn print(
    stdout: &mut std::io::StdoutLock,
    value: &Value,
    pretty: bool,
) -> Result<(), std::io::Error> {
    if value.is_null() {
        return Ok(());
    }
    if pretty {
        writeln!(stdout, "{}", to_string_pretty(value)?)?;
    } else {
        writeln!(stdout, "{}", value.to_string())?;
    }
    Ok(())
}

fn main() {
    let matches = App::new("pbz2jsonl")
        .about("PBZ to JSONL converter")
        .arg(
            Arg::with_name("selector")
                .short("x")
                .help("Looks up a value by a JSON path or pointer")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("skip")
                .short("s")
                .help("Number of ojects to skip")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("take")
                .short("t")
                .help("Number of ojects to take")
                .takes_value(true),
        )
        .arg(Arg::with_name("pretty").short("p").help("Pretty printing"))
        .arg(
            Arg::with_name("PATH")
                .help("Path to pbz file to parse")
                .required(true),
        )
        .get_matches();

    let mut use_json_path = false;
    let mut use_json_pointer = false;
    let selector = matches.value_of("selector").unwrap_or("");
    match selector.get(..1) {
        Some("$") => use_json_path = true,
        Some("/") => use_json_pointer = true,
        _ => (),
    };

    let mut skip = matches
        .value_of("skip")
        .unwrap_or("0")
        .parse::<i32>()
        .unwrap();

    let mut take = matches
        .value_of("take")
        .unwrap_or("0")
        .parse::<i32>()
        .unwrap();

    let pretty = matches.is_present("pretty");

    let filename = matches.value_of("PATH").unwrap();
    let mut pbz = PBZReader::new(&filename).unwrap();

    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    'mainloop: loop {
        let val = pbz.next_value();
        if val.is_err() {
            break;
        }
        if skip > 0 {
            skip -= 1;
            continue;
        }
        let message = val.unwrap();
        if use_json_pointer {
            let pmessage = message.pointer(selector);
            if pmessage.is_some() && print(&mut stdout, pmessage.unwrap(), pretty).is_err() {
                break;
            }
        } else if use_json_path {
            let pmessage = select(&message, selector);
            if pmessage.is_ok() {
                for r in pmessage.unwrap() {
                    if print(&mut stdout, r, pretty).is_err() {
                        break 'mainloop;
                    }
                }
                break;
            }
        } else {
            if print(&mut stdout, &message, pretty).is_err() {
                break;
            }
        }
        if take > 0 {
            take -= 1;
            if take == 0 {
                break;
            }
        }
    }
}
