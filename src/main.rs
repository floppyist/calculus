mod core;

fn main() {
    let test_input = "(3.14 * sin(x * 2) + 42) / (5 ^ 2 ^ 3 - tan(y / 0.5)) * (x + y) ^ (2 * 3) - 0.001 * cos(100 - x)*sqrt(z)";
    let tokens = core::tokenizer::tokenize(test_input);

    println!("\n\nInput: {}", test_input);
    print!("Result: ");

    match tokens {
        Ok(token) => {
            println!("{:?}", token);

            let ast = core::parser::generate_ast(token);

            println!("{:?}", ast);

            let variables = vec![('x', 3.1), ('y', 2.0), ('z', 4.2)];

            let sum = ast.evaluate(&variables);

            println!("Variables: {:?}", variables);

            println!("Sum: {}", sum);
        }
        Err(err) => {
            print!("{}", err)
        }
    }

    println!();
}
