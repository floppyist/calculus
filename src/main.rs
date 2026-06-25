mod core;
mod tui;

fn main() {
    let test_input = "3sin(x + co*sss(y - tan(z) + 7)/10)";
    let tokens = core::tokenizer::tokenize(test_input);

    println!("\n\nInput: {}", test_input);
    print!("Result: ");

    match tokens {
        Ok(token) => {
            println!("{:?}", token);

            let ast = core::parser::generate_ast(token);

            println!("{:?}", ast);
        },
        Err(err) => {
            print!("{}", err)
        }
    }


    println!();
}
