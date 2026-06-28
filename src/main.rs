use std::io::Write;
use std::io::stdin;
use std::io::{self};

mod core;

fn main() -> io::Result<()> {
    let mut buffer = String::new();
    let stdin = stdin();

    println!("Calculus");

    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        stdin.read_line(&mut buffer)?;

        if buffer.is_empty() {
            println!("[INPUT]:\t{}", &buffer);
        }

        let tokens = core::tokenizer::tokenize(&buffer);
        let variables = vec![('x', 3.1), ('y', 2.0), ('z', 4.2)];

        match tokens {
            Ok(token) => {
                println!("[TOKENS]:\t{:?}", token);

                let ast = core::parser::generate_ast(token);

                println!("[AST]:\t\t{:?}", ast);

                let sum = ast.evaluate(&variables);

                println!("[VARIABLES]:\t{:?}", variables);
                println!("[SUM]:\t\t{}", sum);
            }

            Err(err) => {
                println!("{}", err)
            }
        }

        buffer.clear();
    }
}
