#[derive(Debug)]
pub enum Node {
    IntNumber(i64),
    FloatNumber(f64),

    Variable(char),

    Add(Box<Node>, Box<Node>),
    Sub(Box<Node>, Box<Node>),
    Mul(Box<Node>, Box<Node>),
    Div(Box<Node>, Box<Node>),

    Pow(Box<Node>, Box<Node>),

    Sin(Box<Node>),
    Cos(Box<Node>),
    Tan(Box<Node>),
}

impl Node {
    pub fn evaluate(&self, variables: &Vec<(char, f64)>) -> f64 {
        match self {
            Node::IntNumber(n) => *n as f64,

            Node::FloatNumber(n) => *n,

            Node::Variable(c) => variables
                .iter()
                .find(|(n, _)| n == c)
                .map(|(_, v)| *v)
                .unwrap_or(0.0),

            Node::Add(left, right) => left.evaluate(variables) + right.evaluate(variables),
            Node::Sub(left, right) => left.evaluate(variables) - right.evaluate(variables),
            Node::Mul(left, right) => left.evaluate(variables) * right.evaluate(variables),
            Node::Div(left, right) => left.evaluate(variables) / right.evaluate(variables),
            Node::Pow(left, right) => left.evaluate(variables).powf(right.evaluate(variables)),

            Node::Sin(left) => left.evaluate(variables).sin(),
            Node::Cos(left) => left.evaluate(variables).cos(),
            Node::Tan(left) => left.evaluate(variables).tan(),
        }
    }
}
