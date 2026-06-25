mod core;

fn main() {
    // let test_input = "3*sin(x + y*cos(z - tan(a + b*cos(c - sin(d*e + f*g/2))) * 7) / (2*x + 4*y)) - (5*cos(x*y) + tan(z - 1) / (x + y + z)) * 2*sin(cos(tan(x))) + 8/(2/(2/(2/(x + 1))))";
    //let test_input = "3sin(x + co*sss(y - tan(z) + 7)/10)";
    let test_input = "3+4^7+x-3";
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
