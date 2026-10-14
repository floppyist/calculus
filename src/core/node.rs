use std::collections::HashMap;

use crate::core::Function;

// Maximum call depth so recursive user functions cannot blow the stack
const MAX_CALL_DEPTH: usize = 256;

#[derive(Debug, Clone)]
pub enum Node {
    IntNumber(i64),
    FloatNumber(f64),

    Variable(char),

    Add(Box<Node>, Box<Node>),
    Sub(Box<Node>, Box<Node>),
    Mul(Box<Node>, Box<Node>),
    Div(Box<Node>, Box<Node>),

    Pow(Box<Node>, Box<Node>),

    // Unary minus
    Neg(Box<Node>),

    // Built-in function call: function + arguments
    Builtin(Function, Vec<Node>),

    // User function call: name + arguments
    Call(String, Vec<Node>),
}

// Evaluation context: current variable values and known functions
pub struct EvalContext<'a> {
    pub variables: &'a HashMap<char, f64>,
    pub functions: &'a HashMap<String, (Vec<char>, Node)>,
    pub depth: usize,
}

impl Node {
    pub fn evaluate(&self, ctx: &EvalContext) -> f64 {
        match self {
            Node::IntNumber(n) => *n as f64,

            Node::FloatNumber(n) => *n,

            Node::Variable(c) => ctx.variables.get(c).copied().unwrap_or(0.0),

            Node::Add(left, right) => left.evaluate(ctx) + right.evaluate(ctx),
            Node::Sub(left, right) => left.evaluate(ctx) - right.evaluate(ctx),
            Node::Mul(left, right) => left.evaluate(ctx) * right.evaluate(ctx),
            Node::Div(left, right) => left.evaluate(ctx) / right.evaluate(ctx),
            Node::Pow(left, right) => left.evaluate(ctx).powf(right.evaluate(ctx)),

            Node::Neg(inner) => -inner.evaluate(ctx),

            Node::Builtin(func, arguments) => {
                let values: Vec<f64> = arguments
                    .iter()
                    .map(|argument| argument.evaluate(ctx))
                    .collect();

                func.eval(&values)
            }

            Node::Call(name, arguments) => {
                if ctx.depth >= MAX_CALL_DEPTH {
                    return f64::NAN;
                }

                let Some((parameters, body)) = ctx.functions.get(name) else {
                    return f64::NAN;
                };

                // Arguments are evaluated in the caller's context
                let evaluated: Vec<f64> = arguments.iter().map(|arg| arg.evaluate(ctx)).collect();

                // Parameters shadow global variables
                let mut local = ctx.variables.clone();

                for (parameter, value) in parameters.iter().zip(evaluated.iter()) {
                    local.insert(*parameter, *value);
                }

                let child = EvalContext {
                    variables: &local,
                    functions: ctx.functions,
                    depth: ctx.depth + 1,
                };

                body.evaluate(&child)
            }
        }
    }
}
