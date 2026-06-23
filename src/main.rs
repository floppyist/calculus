use std::io;

mod core;

fn main() {
    let mut buffer = String::new();
    let stdin = io::stdin();

    let _ = stdin.read_line(&mut buffer);

    let tokens = core::tokenizer::tokenize(&buffer);

    println!("\n\nInput: {}", buffer);
    print!("Result: ");

    match tokens {
        Ok(token) => print!("{:?}", token),
        Err(err) => {
            print!("{}", err)
        }
    }

    println!();
}
