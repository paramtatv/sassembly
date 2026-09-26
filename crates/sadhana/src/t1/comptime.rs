use crate::t1::ast::*;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Int(i64),
    Float(u64), // Simple bits representation
    Bool(bool),
    String(String),
}

#[derive(Debug, Clone)]
pub struct ComptimeError {
    pub reason: String,
}

pub struct Evaluator {
    fuel: usize,
    env: HashMap<String, Value>, // Global/constant environment
}

impl Evaluator {
    pub fn new(fuel_limit: usize) -> Self {
        Self {
            fuel: fuel_limit,
            env: HashMap::new(),
        }
    }

    fn consume_fuel(&mut self, amount: usize) -> Result<(), ComptimeError> {
        if self.fuel < amount {
            return Err(ComptimeError {
                reason: "Comptime execution exceeded fuel limit".into(),
            });
        }
        self.fuel -= amount;
        Ok(())
    }

    pub fn eval_expression(&mut self, expr: &Expression) -> Result<Value, ComptimeError> {
        self.consume_fuel(1)?;

        match expr {
            Expression::Identifier(name) => {
                if let Some(val) = self.env.get(name) {
                    Ok(val.clone())
                } else {
                    Err(ComptimeError {
                        reason: format!("Undefined comptime variable: {}", name),
                    })
                }
            }
            Expression::Numeral(text) => {
                // In a real compiler, we parse standard integers
                if let Ok(val) = text.parse::<i64>() {
                    Ok(Value::Int(val))
                } else {
                    Err(ComptimeError {
                        reason: format!("Failed to parse numeral at comptime: {}", text),
                    })
                }
            }
            Expression::StringLiteral(text) => Ok(Value::String(text.clone())),
            Expression::Group(inner) => self.eval_expression(inner),
            Expression::Index(_) => Err(ComptimeError {
                reason: "Comptime index operations not fully supported".into(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_constant() {
        let mut evaluator = Evaluator::new(100);
        let result = evaluator.eval_expression(&Expression::Numeral("42".into()));
        assert_eq!(result.unwrap(), Value::Int(42));
    }

    #[test]
    fn test_fuel_exhaustion() {
        let mut evaluator = Evaluator::new(1);
        let expr = Expression::Group(Box::new(Expression::Numeral("42".into())));

        // Group evaluates inner, so it takes 1 fuel for Group, and 1 for Numeral = 2 total.
        let result = evaluator.eval_expression(&expr);
        assert!(result.is_err());
        assert!(result.unwrap_err().reason.contains("exceeded fuel limit"));
    }
}
