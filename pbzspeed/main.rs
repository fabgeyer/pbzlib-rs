use clap::{App, Arg};
use indicatif::{ProgressBar, ProgressStyle};
use pbzlib::PBZReader;

fn main() {
    let matches = App::new("pbzspeed")
        .arg(
            Arg::with_name("PATH")
                .help("Path to pbz file to parse")
                .required(true),
        )
        .get_matches();

    let filename = matches.value_of("PATH").unwrap();
    let mut pbz = PBZReader::new(&filename).unwrap();

    let pbar = ProgressBar::new_spinner();
    pbar.set_style(ProgressStyle::template(
        ProgressStyle::default_spinner(),
        "{spinner} {pos}",
    ));
    loop {
        let message = pbz.next_value();
        if message.is_err() {
            break;
        }
        pbar.inc(1);
    }
    pbar.finish();
}
