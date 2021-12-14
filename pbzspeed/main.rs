use pbzlib::PBZReader;

fn main() {
    let filename = std::env::args().nth(1).expect("no filename given");
    let mut pbz = PBZReader::new(&filename).unwrap();

    loop {
        let message = pbz.next_value();
        if message.is_err() {
            break;
        }
    }
}
