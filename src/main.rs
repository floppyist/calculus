mod core;

fn main() {
    let test_input = "3sin(x + co*sss(y - tan(z) + 7).00)";
    let tokens = core::tokenizer::tokenize(test_input);

    println!("\n\nInput: {}", test_input);
    print!("Result: ");

    match tokens {
        Ok(token) => print!("{:?}", token),
        Err(err) => {
            print!("{}", err)
        }
    }

    println!();
}
