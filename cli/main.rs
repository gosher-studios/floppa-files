use std::collections::HashMap;
use std::env;
use std::io::Read;
fn main() {
    let mut meow = Vec::new();
    for argument in env::args() {
        meow.push(argument);
    }
    let mut file = std::fs::File::open("logfile").unwrap();
    let mut output = vec![];
    file.read_to_end(&mut output).unwrap();
    let db: HashMap<String, Vec<String>> = bincode::deserialize(&output).unwrap();

    let mrow: Vec<(String, Vec<String>)> = db
        .into_iter()
        .filter_map(|i| {
            if meow.contains(&i.0)
                | (&i
                    .clone()
                    .1
                    .into_iter()
                    .filter_map(|s| if meow.contains(&s) { Some(s) } else { None })
                    .count()
                    >= &1)
            {
                Some(i)
            } else {
                None
            }
        })
        .collect();
    for item in mrow {
        println!("{:?}, {:?}", item.0, item.1);
    }
}
