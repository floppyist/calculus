use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::core::Token;
use crate::core::algebra;
use crate::core::node::{EvalContext, Node};
use crate::core::parser::{ParseError, generate_ast};
use crate::core::tokenizer::{TokenizerError, tokenize, tokenize_definition_side};

// Names auto assigned to plotted curves ('e' is reserved for Euler's number)
const AUTO_NAMES: &str = "fghijklmnopqrstuvwxyzabcd";

// A definition (variable or function) with its source text for display
#[derive(Debug)]
pub struct Definition {
    pub source: String,
    pub params: Vec<char>,
    pub body: Node,
}

#[derive(Debug)]
pub enum EvalError {
    Tokenizer(TokenizerError),
    Parse(ParseError),
    InvalidAssignment,
    UnknownVariable(char),
    FunctionNotDefined(String),
    WrongArgumentCount(String),
    CyclicDependency,
    NotDifferentiable,
    NotIntegrable,
    NotInvertible,
}

impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvalError::Tokenizer(err) => write!(f, "{}", err),
            EvalError::Parse(err) => write!(f, "{}", err),
            EvalError::InvalidAssignment => write!(f, "[ERROR]: Invalid assignment"),
            EvalError::UnknownVariable(name) => {
                write!(f, "[ERROR]: Variable '{}' is not defined", name)
            }
            EvalError::FunctionNotDefined(name) => {
                write!(f, "[ERROR]: Function '{}' is not defined", name)
            }
            EvalError::WrongArgumentCount(name) => {
                write!(
                    f,
                    "[ERROR]: Function '{}' was called with the wrong number of arguments",
                    name
                )
            }
            EvalError::CyclicDependency => write!(f, "[ERROR]: Cyclic variable dependency"),
            EvalError::NotDifferentiable => {
                write!(f, "[ERROR]: Expression can not be differentiated")
            }
            EvalError::NotIntegrable => {
                write!(f, "[ERROR]: No symbolic antiderivative found")
            }
            EvalError::NotInvertible => {
                write!(f, "[ERROR]: Expression has no known inverse")
            }
        }
    }
}

impl From<TokenizerError> for EvalError {
    fn from(error: TokenizerError) -> Self {
        EvalError::Tokenizer(error)
    }
}

impl From<ParseError> for EvalError {
    fn from(error: ParseError) -> Self {
        EvalError::Parse(error)
    }
}

#[derive(Debug)]
pub enum Outcome {
    Value(f64),
    VariableDefined(char),
    FunctionDefined(String),
    Source(String),
}

// Result of an integration: a new function or a definite value
#[derive(Debug)]
pub enum IntegralOutcome {
    Function { name: String, source: String },
    Value(f64),
}

// Target of an assignment
enum Target {
    Variable(char),
    Function(String, Vec<char>),
}

// Runtime environment: definitions and variable values derived from them
pub struct Environment {
    variables: HashMap<char, Definition>,
    functions: HashMap<String, Definition>,
    values: HashMap<char, f64>,
}

impl Environment {
    pub fn new() -> Self {
        Environment {
            variables: HashMap::new(),
            functions: HashMap::new(),
            values: HashMap::new(),
        }
    }

    pub fn functions(&self) -> &HashMap<String, Definition> {
        &self.functions
    }

    pub fn values(&self) -> &HashMap<char, f64> {
        &self.values
    }

    // Next free function name: f..z, a..d, then f2, g2, ... ('e' is reserved for Euler's number)
    pub fn next_function_name(&self) -> String {
        for name in AUTO_NAMES.chars() {
            if !self.functions.contains_key(&name.to_string()) {
                return name.to_string();
            }
        }

        let mut suffix = 2;

        loop {
            for name in AUTO_NAMES.chars() {
                let candidate = format!("{}{}", name, suffix);

                if !self.functions.contains_key(&candidate) {
                    return candidate;
                }
            }

            suffix += 1;
        }
    }

    // Variable definitions as "name = expression"
    pub fn variable_definitions(&self) -> Vec<String> {
        let mut names: Vec<char> = self.variables.keys().copied().collect();

        names.sort();

        names
            .iter()
            .map(|name| format!("{} = {}", name, self.variables[name].source))
            .collect()
    }

    // Function definitions as "name(params) = expression"
    pub fn function_definitions(&self) -> Vec<String> {
        let mut names: Vec<String> = self.functions.keys().cloned().collect();

        names.sort();

        names
            .iter()
            .map(|name| {
                let definition = &self.functions[name];
                let params: String = definition.params.iter().copied().collect();

                format!("{}({}) = {}", name, params, definition.source)
            })
            .collect()
    }

    // Removes a user function; true if it existed
    pub fn remove_function(&mut self, name: &str) -> bool {
        self.functions.remove(name).is_some()
    }

    // Removes a variable and recomputes dependents; true if it existed
    pub fn remove_variable(&mut self, name: char) -> bool {
        if self.variables.remove(&name).is_none() {
            return false;
        }

        self.values.remove(&name);
        let _ = self.recompute();

        true
    }

    // Removes every variable
    pub fn clear_variables(&mut self) {
        self.variables.clear();
        self.values.clear();
    }

    // Removes every user function and recomputes variables
    pub fn clear_functions(&mut self) {
        self.functions.clear();
        let _ = self.recompute();
    }

    // Function value once all parameters are set as variables
    pub fn function_result(&self, definition: &Definition) -> Option<f64> {
        let all_set = definition
            .params
            .iter()
            .all(|param| self.values.contains_key(param));

        if !all_set {
            return None;
        }

        let functions = self.function_map();
        let ctx = EvalContext {
            variables: &self.values,
            functions: &functions,
            depth: 0,
        };

        Some(definition.body.evaluate(&ctx))
    }

    // Parses an expression and validates all function calls
    pub fn expression(&self, expression: &str) -> Result<Node, EvalError> {
        self.parse_and_validate(expression, &[])
    }

    // Simplified source text of an expression
    pub fn simplify(&self, expression: &str) -> Result<String, EvalError> {
        let ast = self.parse_and_validate(expression, &[])?;

        Ok(algebra::to_source(&algebra::simplify(&ast)))
    }

    // Differentiates an expression and stores it as a new one-parameter function
    pub fn differentiate(&mut self, expression: &str) -> Result<(String, String), EvalError> {
        let ast = algebra::simplify(&self.parse_and_validate(expression, &[])?);
        let functions = self.function_map();
        let derivative =
            algebra::differentiate(&ast, 'x', &functions).ok_or(EvalError::NotDifferentiable)?;
        let simplified = algebra::simplify(&derivative);

        Ok(self.define_function(&simplified))
    }

    // Integrates an expression; bounds make it definite, otherwise a function is stored
    pub fn integrate(
        &mut self,
        expression: &str,
        bounds: Option<(&str, &str)>,
    ) -> Result<IntegralOutcome, EvalError> {
        let ast = algebra::simplify(&self.parse_and_validate(expression, &[])?);

        match bounds {
            None => {
                let antiderivative =
                    algebra::integrate(&ast, 'x').ok_or(EvalError::NotIntegrable)?;
                let simplified = algebra::simplify(&antiderivative);
                let (name, source) = self.define_function(&simplified);

                Ok(IntegralOutcome::Function { name, source })
            }

            Some((lower, upper)) => {
                let lower = self.evaluate_constant(lower)?;
                let upper = self.evaluate_constant(upper)?;

                let value = match algebra::integrate(&ast, 'x') {
                    Some(antiderivative) => {
                        let simplified = algebra::simplify(&antiderivative);

                        self.evaluate_at(&simplified, 'x', upper)
                            - self.evaluate_at(&simplified, 'x', lower)
                    }
                    None => self.simpson(&ast, lower, upper),
                };

                Ok(IntegralOutcome::Value(value))
            }
        }
    }

    // Stores a single-variable expression as a new auto-named function
    fn define_function(&mut self, body: &Node) -> (String, String) {
        let source = algebra::to_source(body);
        let name = self.next_function_name();

        self.functions.insert(
            name.clone(),
            Definition {
                source: source.clone(),
                params: vec!['x'],
                body: body.clone(),
            },
        );

        let _ = self.recompute();

        (name, source)
    }

    // Evaluates a constant expression using the current variable values
    fn evaluate_constant(&self, expression: &str) -> Result<f64, EvalError> {
        let ast = self.parse_and_validate(expression, &[])?;

        if let Some(name) = self.unknown_variable(&ast) {
            return Err(EvalError::UnknownVariable(name));
        }

        Ok(self.evaluate_at(&ast, 'x', 0.0))
    }

    // Evaluates an expression with one variable bound to a value
    fn evaluate_at(&self, node: &Node, name: char, value: f64) -> f64 {
        let functions = self.function_map();
        let mut variables = self.values.clone();

        variables.insert(name, value);

        let ctx = EvalContext {
            variables: &variables,
            functions: &functions,
            depth: 0,
        };

        node.evaluate(&ctx)
    }

    // Composite Simpson rule, used when no antiderivative is available
    fn simpson(&self, node: &Node, lower: f64, upper: f64) -> f64 {
        let steps = 1000;
        let width = (upper - lower) / steps as f64;
        let mut total = self.evaluate_at(node, 'x', lower) + self.evaluate_at(node, 'x', upper);

        for index in 1..steps {
            let sample = lower + width * index as f64;
            let weight = if index % 2 == 1 { 4.0 } else { 2.0 };

            total += weight * self.evaluate_at(node, 'x', sample);
        }

        total * width / 3.0
    }

    pub fn execute(&mut self, raw_input: &str) -> Result<Outcome, EvalError> {
        let input = raw_input.trim();

        let mut parts = input.split('=');
        let first = parts.next().unwrap_or("");

        match parts.next() {
            None => self.evaluate_expression(first.trim()),
            Some(second) => {
                // More than one '=' is invalid
                if parts.next().is_some() {
                    return Err(EvalError::InvalidAssignment);
                }

                self.apply_assignment(first, second)
            }
        }
    }

    fn evaluate_expression(&self, expression: &str) -> Result<Outcome, EvalError> {
        let ast = self.parse_and_validate(expression, &[])?;

        if let Some(name) = self.unknown_variable(&ast) {
            if let Some(source) = self.inverse_source(expression)? {
                return Ok(Outcome::Source(source));
            }

            return Err(EvalError::UnknownVariable(name));
        }

        let functions = self.function_map();
        let ctx = EvalContext {
            variables: &self.values,
            functions: &functions,
            depth: 0,
        };

        Ok(Outcome::Value(ast.evaluate(&ctx)))
    }

    // Symbolic inverse of a bare call(v)^-1, printed with the variable y
    fn inverse_source(&self, expression: &str) -> Result<Option<String>, EvalError> {
        let ast = self.parse_ast(expression, &[])?;
        let Node::Pow(base, exponent) = &ast else {
            return Ok(None);
        };

        if !is_invers_exponent(exponent) {
            return Ok(None);
        }

        let functions = self.function_map();
        let inverted = match base.as_ref() {
            Node::Builtin(function, arguments) => match arguments.as_slice() {
                [Node::Variable(variable)] => {
                    algebra::builtin_inverse(*function, arguments[0].clone())
                        .map(|node| (node, *variable))
                }
                _ => None,
            },
            Node::Call(name, arguments) => match arguments.as_slice() {
                [Node::Variable(variable)] => functions.get(name).and_then(|(parameters, body)| {
                    parameters.first().and_then(|param| {
                        algebra::user_inverse(body, *param, arguments[0].clone())
                            .map(|node| (node, *variable))
                    })
                }),
                _ => None,
            },
            _ => None,
        };

        let Some((inverted, variable)) = inverted else {
            return Ok(None);
        };

        let mut replacements = HashMap::new();
        replacements.insert(variable, Node::Variable('y'));

        Ok(Some(algebra::to_source(&algebra::simplify(
            &algebra::substitute(&inverted, &replacements),
        ))))
    }

    // First free variable of the expression that is not defined, if any
    fn unknown_variable(&self, node: &Node) -> Option<char> {
        let functions = self.function_map();
        let mut collected = HashSet::new();
        let mut visited_functions = HashSet::new();

        collect_vars(
            node,
            &functions,
            &HashSet::new(),
            &mut visited_functions,
            &mut collected,
        );

        let mut names: Vec<char> = collected
            .into_iter()
            .filter(|name| !self.values.contains_key(name))
            .collect();

        names.sort();
        names.into_iter().next()
    }

    fn apply_assignment(&mut self, left: &str, right: &str) -> Result<Outcome, EvalError> {
        // A function signature is only ever a target on the left side; the
        // right side is an expression (a bare f(x) there is a call, not a target)
        let left_target = parse_target(left, true)?;
        let right_target = parse_target(right, false)?;

        match (left_target, right_target) {
            // Exactly one side must be the target
            (Some(_), Some(_)) | (None, None) => Err(EvalError::InvalidAssignment),
            (Some(target), None) => self.define(target, right.trim()),
            (None, Some(target)) => self.define(target, left.trim()),
        }
    }

    fn define(&mut self, target: Target, expression: &str) -> Result<Outcome, EvalError> {
        match target {
            Target::Variable(name) => {
                let body = self.parse_and_validate(expression, &[])?;

                self.variables.insert(
                    name,
                    Definition {
                        source: expression.to_string(),
                        params: Vec::new(),
                        body,
                    },
                );

                self.recompute()?;

                Ok(Outcome::VariableDefined(name))
            }

            Target::Function(name, params) => {
                let unique: HashSet<char> = params.iter().copied().collect();

                if unique.len() != params.len() {
                    return Err(EvalError::InvalidAssignment);
                }

                // Own name is known while parsing the body (recursion)
                let body = self.parse_and_validate(expression, &[(name.clone(), params.len())])?;

                // Store the compact form so every display shows the same source
                let body = algebra::simplify(&body);
                let source = algebra::to_source(&body);

                self.functions.insert(
                    name.clone(),
                    Definition {
                        source,
                        params,
                        body,
                    },
                );

                self.recompute()?;

                Ok(Outcome::FunctionDefined(name))
            }
        }
    }

    // Tokenizes and parses an expression and validates all function calls
    fn parse_and_validate(
        &self,
        expression: &str,
        extra_functions: &[(String, usize)],
    ) -> Result<Node, EvalError> {
        let ast = self.parse_ast(expression, extra_functions)?;

        rewrite_inverses(&ast, &self.function_map())
    }

    // Tokenize, parse and validate without rewriting inverse powers
    fn parse_ast(
        &self,
        expression: &str,
        extra_functions: &[(String, usize)],
    ) -> Result<Node, EvalError> {
        let mut names: Vec<String> = self.functions.keys().cloned().collect();
        let mut arities: HashMap<String, usize> = self
            .functions
            .iter()
            .map(|(name, definition)| (name.clone(), definition.params.len()))
            .collect();

        for (name, arity) in extra_functions {
            names.push(name.clone());
            arities.insert(name.clone(), *arity);
        }

        let tokens = tokenize(expression, &names)?;
        let ast = generate_ast(tokens)?;

        validate_calls(&ast, &arities)?;

        Ok(ast)
    }

    pub fn function_map(&self) -> HashMap<String, (Vec<char>, Node)> {
        self.functions
            .iter()
            .map(|(name, definition)| {
                (
                    name.clone(),
                    (definition.params.clone(), definition.body.clone()),
                )
            })
            .collect()
    }

    // Recomputes all variable values in dependency order
    fn recompute(&mut self) -> Result<(), EvalError> {
        let order = self.topological_order()?;
        let functions = self.function_map();
        let mut values: HashMap<char, f64> = HashMap::new();

        for name in &order {
            let value = {
                let definition = self.variables.get(name).expect("variable must exist");

                let ctx = EvalContext {
                    variables: &values,
                    functions: &functions,
                    depth: 0,
                };

                definition.body.evaluate(&ctx)
            };

            values.insert(*name, value);
        }

        self.values = values;

        Ok(())
    }

    // Topological sort of variables based on their free variables
    fn topological_order(&self) -> Result<Vec<char>, EvalError> {
        let functions = self.function_map();

        let mut dependencies: HashMap<char, HashSet<char>> = HashMap::new();

        for (name, definition) in &self.variables {
            let mut collected = HashSet::new();
            let mut visited_functions = HashSet::new();

            collect_vars(
                &definition.body,
                &functions,
                &HashSet::new(),
                &mut visited_functions,
                &mut collected,
            );

            dependencies.insert(*name, collected);
        }

        let mut indegree: HashMap<char, usize> =
            self.variables.keys().map(|name| (*name, 0)).collect();
        let mut dependents: HashMap<char, Vec<char>> = HashMap::new();

        for (name, deps) in &dependencies {
            for dependency in deps {
                if self.variables.contains_key(dependency) {
                    *indegree.get_mut(name).expect("variable must exist") += 1;
                    dependents.entry(*dependency).or_default().push(*name);
                }
            }
        }

        let mut queue: Vec<char> = indegree
            .iter()
            .filter(|(_, degree)| **degree == 0)
            .map(|(name, _)| *name)
            .collect();
        let mut order = Vec::new();

        while let Some(name) = queue.pop() {
            order.push(name);

            if let Some(children) = dependents.get(&name) {
                for child in children {
                    let degree = indegree.get_mut(child).expect("variable must exist");

                    *degree -= 1;

                    if *degree == 0 {
                        queue.push(*child);
                    }
                }
            }
        }

        if order.len() != self.variables.len() {
            return Err(EvalError::CyclicDependency);
        }

        Ok(order)
    }
}

// Determines the target of one side: single variable, or a function signature on the left
fn parse_target(side: &str, allow_function: bool) -> Result<Option<Target>, EvalError> {
    let trimmed = side.trim();

    if trimmed.is_empty() {
        return Ok(None);
    }

    let tokens = tokenize_definition_side(trimmed)?;

    if let [Token::Variable(name)] = tokens.as_slice() {
        return Ok(Some(Target::Variable(*name)));
    }

    if !allow_function {
        return Ok(None);
    }

    Ok(parse_function_signature(&tokens))
}

fn parse_function_signature(tokens: &[Token]) -> Option<Target> {
    let mut iter = tokens.iter().peekable();

    let name = match iter.next() {
        Some(Token::UserFunction(name)) => name.clone(),
        _ => return None,
    };

    match iter.next() {
        Some(Token::Lparen) => {}
        _ => return None,
    }

    let mut params = Vec::new();

    // Empty parameter list is allowed
    if let Some(Token::Rparen) = iter.peek() {
        iter.next();

        return if iter.next().is_none() {
            Some(Target::Function(name, params))
        } else {
            None
        };
    }

    loop {
        match iter.next() {
            Some(Token::Variable(parameter)) => params.push(*parameter),
            _ => return None,
        }

        match iter.next() {
            Some(Token::Comma) => continue,
            Some(Token::Rparen) => break,
            _ => return None,
        }
    }

    // The whole side must be exactly the signature, not just a prefix of it
    if iter.next().is_none() {
        Some(Target::Function(name, params))
    } else {
        None
    }
}

// Rewrites call(x)^-1 into the inverse function; x^-1 keeps meaning 1/x
fn rewrite_inverses(
    node: &Node,
    functions: &HashMap<String, (Vec<char>, Node)>,
) -> Result<Node, EvalError> {
    match node {
        Node::IntNumber(_) | Node::FloatNumber(_) | Node::Variable(_) => Ok(node.clone()),

        Node::Neg(inner) => Ok(Node::Neg(Box::new(rewrite_inverses(inner, functions)?))),

        Node::Add(left, right) => Ok(Node::Add(
            Box::new(rewrite_inverses(left, functions)?),
            Box::new(rewrite_inverses(right, functions)?),
        )),
        Node::Sub(left, right) => Ok(Node::Sub(
            Box::new(rewrite_inverses(left, functions)?),
            Box::new(rewrite_inverses(right, functions)?),
        )),
        Node::Mul(left, right) => Ok(Node::Mul(
            Box::new(rewrite_inverses(left, functions)?),
            Box::new(rewrite_inverses(right, functions)?),
        )),
        Node::Div(left, right) => Ok(Node::Div(
            Box::new(rewrite_inverses(left, functions)?),
            Box::new(rewrite_inverses(right, functions)?),
        )),

        Node::Pow(base, exponent) => {
            let base = rewrite_inverses(base, functions)?;

            if is_invers_exponent(exponent) {
                return match &base {
                    Node::Builtin(function, arguments) if arguments.len() == 1 => {
                        algebra::builtin_inverse(*function, arguments[0].clone())
                            .ok_or(EvalError::NotInvertible)
                    }
                    Node::Call(name, arguments) if arguments.len() == 1 => {
                        let Some((parameters, body)) = functions.get(name) else {
                            return Err(EvalError::NotInvertible);
                        };
                        let Some(&param) = parameters.first() else {
                            return Err(EvalError::NotInvertible);
                        };

                        algebra::user_inverse(body, param, arguments[0].clone())
                            .ok_or(EvalError::NotInvertible)
                    }
                    Node::Builtin(..) | Node::Call(..) => Err(EvalError::NotInvertible),
                    _ => Ok(Node::Pow(
                        Box::new(base),
                        Box::new(Node::Neg(Box::new(Node::IntNumber(1)))),
                    )),
                };
            }

            Ok(Node::Pow(
                Box::new(base),
                Box::new(rewrite_inverses(exponent, functions)?),
            ))
        }

        Node::Builtin(function, arguments) => {
            let rewritten: Result<Vec<Node>, EvalError> = arguments
                .iter()
                .map(|argument| rewrite_inverses(argument, functions))
                .collect();

            Ok(Node::Builtin(*function, rewritten?))
        }
        Node::Call(name, arguments) => {
            let rewritten: Result<Vec<Node>, EvalError> = arguments
                .iter()
                .map(|argument| rewrite_inverses(argument, functions))
                .collect();

            Ok(Node::Call(name.clone(), rewritten?))
        }
    }
}

// True for an exponent that denotes the inverse function: ^-1
fn is_invers_exponent(node: &Node) -> bool {
    match node {
        Node::Neg(inner) => match inner.as_ref() {
            Node::IntNumber(value) => *value == 1,
            Node::FloatNumber(value) => *value == 1.0,
            _ => false,
        },
        Node::IntNumber(value) => *value == -1,
        Node::FloatNumber(value) => *value == -1.0,
        _ => false,
    }
}

// Validates that all calls match known functions with the right argument count
fn validate_calls(node: &Node, arities: &HashMap<String, usize>) -> Result<(), EvalError> {
    match node {
        Node::Builtin(func, arguments) => {
            if !func.arities().contains(&arguments.len()) {
                return Err(EvalError::WrongArgumentCount(func.name().to_string()));
            }

            for argument in arguments {
                validate_calls(argument, arities)?;
            }

            Ok(())
        }

        Node::Call(name, arguments) => {
            let expected = arities
                .get(name)
                .ok_or_else(|| EvalError::FunctionNotDefined(name.clone()))?;

            if *expected != arguments.len() {
                return Err(EvalError::WrongArgumentCount(name.clone()));
            }

            for argument in arguments {
                validate_calls(argument, arities)?;
            }

            Ok(())
        }

        Node::Neg(inner) => validate_calls(inner, arities),

        Node::Add(left, right)
        | Node::Sub(left, right)
        | Node::Mul(left, right)
        | Node::Div(left, right)
        | Node::Pow(left, right) => {
            validate_calls(left, arities)?;
            validate_calls(right, arities)
        }

        _ => Ok(()),
    }
}

// Collects free variables; parameters of called functions count as bound
fn collect_vars(
    node: &Node,
    functions: &HashMap<String, (Vec<char>, Node)>,
    bound: &HashSet<char>,
    visited_functions: &mut HashSet<String>,
    out: &mut HashSet<char>,
) {
    match node {
        Node::Variable(name) => {
            if !bound.contains(name) {
                out.insert(*name);
            }
        }

        Node::Add(left, right)
        | Node::Sub(left, right)
        | Node::Mul(left, right)
        | Node::Div(left, right)
        | Node::Pow(left, right) => {
            collect_vars(left, functions, bound, visited_functions, out);
            collect_vars(right, functions, bound, visited_functions, out);
        }

        Node::Neg(inner) => {
            collect_vars(inner, functions, bound, visited_functions, out);
        }

        Node::Builtin(_, arguments) => {
            for argument in arguments {
                collect_vars(argument, functions, bound, visited_functions, out);
            }
        }

        Node::Call(name, arguments) => {
            for argument in arguments {
                collect_vars(argument, functions, bound, visited_functions, out);
            }

            if visited_functions.insert(name.clone())
                && let Some((parameters, body)) = functions.get(name)
            {
                let mut local_bound = bound.clone();

                for parameter in parameters {
                    local_bound.insert(*parameter);
                }

                collect_vars(body, functions, &local_bound, visited_functions, out);
            }
        }

        Node::IntNumber(_) | Node::FloatNumber(_) => {}
    }
}
