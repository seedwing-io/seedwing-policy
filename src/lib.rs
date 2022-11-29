#![allow(unused)]
use std::borrow::Borrow;
use std::cmp::Ordering;
use std::fmt::Debug;

mod lang;

pub const BOOLEAN: Type = Type::Boolean(BooleanType::Any);
pub const INTEGER: Type = Type::Integer(IntegerType::Any);
pub const DECIMAL: Type = Type::Decimal(DecimalType::Any);

#[derive(PartialEq, Clone, Debug)]
pub enum Type {
    Anything,
    Integer(IntegerType),
    Decimal(DecimalType),
    Boolean(BooleanType),
    String(StringType),
    Nothing,
    //
    Join(Box<Type>, Box<Type>),
    Meet(Box<Type>, Box<Type>),
}

impl Type {
    pub fn boolean(value: Option<bool>) -> Type {
        if let Some(value) = value {
            Type::Boolean(value.into())
        } else {
            BOOLEAN
        }
    }

    pub fn integers() -> Type {
        INTEGER
    }

    pub fn integer(value: i64) -> Type {
        Type::Integer(IntegerType::Equal(value))
    }

    pub fn join(&self, other: &Type) -> Type {
        // TODO: optimize
        Type::Join(Box::new(self.clone()), Box::new(other.clone()))
    }

    pub fn meet(&self, other: &Type) -> Type {
        match self.partial_cmp(other) {
            None => self.try_meet(other).unwrap_or(Type::Nothing),
            Some(Ordering::Equal) => self.clone(),
            Some(Ordering::Greater) => self.clone(),
            Some(Ordering::Less) => other.clone(),
        }
    }

    fn try_meet(&self, other: &Type) -> Option<Type> {
        println!("try meet {:?} vs {:?}", self, other);
        match self {
            Type::Anything => Some(other.clone()),
            Type::Integer(c1) => match other {
                Type::Anything => Some(self.clone()),
                Type::Integer(c2) => Some(c1.meet_integer(c2)?),
                Type::Decimal(_) => todo!(),
                Type::Boolean(_) => None,
                Type::String(_) => None,
                Type::Nothing => None,
                Type::Join(_, _) => todo!(),
                Type::Meet(_, _) => todo!(),
            },
            Type::Decimal(_) => todo!(),
            Type::Boolean(_) => todo!(),
            Type::String(_) => todo!(),
            Type::Nothing => todo!(),
            //
            Type::Join(c1, c2) => {
                let r1 = c1.try_meet(other);
                let r2 = c2.try_meet(other);

                match (r1, r2) {
                    (None, None) => None,
                    (Some(_), Some(_)) => {
                        Some(Type::Join(Box::new(self.clone()), Box::new(other.clone())))
                    }
                    (Some(_), None) => Some(Type::Join(Box::new(self.clone()), c1.clone())),
                    (None, Some(_)) => Some(Type::Join(Box::new(self.clone()), c2.clone())),
                }
            }
            Type::Meet(_, _) => todo!(),
        }
    }

    pub fn accepts<T: Borrow<Type>>(&self, other: T) -> bool {
        let result = self.meet(other.borrow());
        !matches!(result, Type::Nothing)
    }
}

impl PartialOrd for Type {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Type::Anything, _) => Some(Ordering::Greater),
            (Type::Integer(lhs), Type::Integer(rhs)) => lhs.partial_cmp(rhs),
            (Type::Decimal(_), _) => todo!(),
            (Type::Boolean(_), _) => todo!(),
            (Type::String(_), _) => todo!(),
            (Type::Nothing, _) => Some(Ordering::Less),
            _ => None,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum IntegerType {
    Any,
    LessThan(i64),
    Equal(i64),
    GreaterThan(i64),
}

impl From<IntegerType> for Type {
    fn from(c: IntegerType) -> Self {
        Self::Integer(c)
    }
}

impl IntegerType {
    pub fn any() -> Type {
        INTEGER
    }

    pub fn equal(value: i64) -> Type {
        Self::Equal(value).into()
    }

    pub fn less_than(value: i64) -> Type {
        Self::LessThan(value).into()
    }

    pub fn less_than_or_equal(value: i64) -> Type {
        Self::LessThan(value).join_integer(&Self::Equal(value))
    }

    pub fn greater_than(value: i64) -> Type {
        Self::GreaterThan(value).into()
    }

    pub fn greater_than_equal(value: i64) -> Type {
        Self::GreaterThan(value).join_integer(&Self::Equal(value))
    }

    fn meet_integer(&self, other: &IntegerType) -> Option<Type> {
        match self {
            IntegerType::Any => Some(Type::Integer(other.clone())),
            IntegerType::LessThan(v1) => {
                match other {
                    IntegerType::Any => Some(Type::Integer(self.clone())),
                    IntegerType::LessThan(v2) => Some(IntegerType::LessThan(*v1.min(v2)).into()),
                    IntegerType::Equal(v2) if v2 < v1 => Some(Type::Integer(other.clone())),
                    IntegerType::Equal(_) => None,
                    IntegerType::GreaterThan(v2) if v1 > v2 => {
                        // MEET
                        todo!()
                    }
                    _ => None,
                }
            }
            IntegerType::Equal(v1) => {
                match other {
                    IntegerType::Any => Some(Type::Integer(self.clone())),
                    IntegerType::LessThan(v2) if v1 < v2 => Some(Type::Integer(self.clone())),
                    IntegerType::Equal(v2) if v1 == v2 => Some(Type::Integer(self.clone())),
                    IntegerType::GreaterThan(v2) => {
                        // MEET
                        todo!()
                    }
                    _ => None,
                }
            }
            IntegerType::GreaterThan(v1) => {
                match other {
                    IntegerType::Any => Some(Type::Integer(self.clone())),
                    IntegerType::LessThan(v2) => {
                        // MEET
                        todo!()
                    }
                    IntegerType::Equal(v2) if v2 > v1 => Some(Type::Integer(other.clone())),
                    IntegerType::GreaterThan(v2) => {
                        Some(IntegerType::GreaterThan(*v1.max(v2)).into())
                    }
                    _ => None,
                }
            }
        }
    }

    fn join_integer(&self, other: &IntegerType) -> Type {
        match self {
            IntegerType::Any => Type::Integer(self.clone()),
            IntegerType::LessThan(v1) | IntegerType::Equal(v1) | IntegerType::GreaterThan(v1) => {
                match self.partial_cmp(other) {
                    None => Type::Join(
                        Box::new(Type::Integer(self.clone())),
                        Box::new(Type::Integer(other.clone())),
                    ),
                    Some(Ordering::Equal) => Type::Integer(self.clone()),
                    Some(Ordering::Less) => Type::Integer(other.clone()),
                    Some(Ordering::Greater) => Type::Integer(self.clone()),
                }
            }
        }
    }
}

impl PartialOrd for IntegerType {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        println!("self {:?} other {:?}", self, other);
        match self {
            IntegerType::Any => Some(Ordering::Greater),
            IntegerType::LessThan(v1) => match other {
                IntegerType::Any => Some(Ordering::Less),
                IntegerType::LessThan(v2) if v1 < v2 => Some(Ordering::Less),
                IntegerType::LessThan(v2) if v1 > v2 => Some(Ordering::Greater),
                IntegerType::LessThan(v2) if v1 == v2 => Some(Ordering::Equal),
                IntegerType::Equal(v2) if v1 > v2 => Some(Ordering::Greater),
                _ => None,
            },
            IntegerType::Equal(v1) => match other {
                IntegerType::Any => Some(Ordering::Less),
                IntegerType::LessThan(v2) if v2 > v1 => Some(Ordering::Less),
                IntegerType::Equal(v2) if v1 == v2 => Some(Ordering::Equal),
                IntegerType::GreaterThan(v2) if v1 > v2 => Some(Ordering::Less),
                _ => None,
            },
            IntegerType::GreaterThan(v1) => other.partial_cmp(self).map(Ordering::reverse),
        }
    }
}

impl PartialEq<DecimalType> for IntegerType {
    fn eq(&self, other: &DecimalType) -> bool {
        todo!()
    }
}

impl PartialOrd<DecimalType> for IntegerType {
    fn partial_cmp(&self, other: &DecimalType) -> Option<Ordering> {
        todo!()
    }
}

#[derive(PartialEq, Clone, Debug)]
pub enum DecimalType {
    Any,
    LessThan(f64),
    LessThanEqual(f64),
    Equal(f64),
    GreaterThanEqual(f64),
    GreaterThan(f64),
}

impl From<DecimalType> for Type {
    fn from(inner: DecimalType) -> Self {
        Self::Decimal(inner)
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum BooleanType {
    Any,
    True,
    False,
}

impl From<bool> for BooleanType {
    fn from(value: bool) -> Self {
        if value {
            Self::True
        } else {
            Self::False
        }
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum StringType {
    Any,
    StartsWith(String),
    Equal(String),
    EndsWith(String),
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::Type::Integer;
    use std::rc::Rc;

    #[test]
    fn integer_constraints() {
        assert!(IntegerType::any() > IntegerType::less_than(42));
        assert!(IntegerType::less_than(42) < IntegerType::any());

        assert!(IntegerType::less_than(42) > IntegerType::less_than(10));
        assert!(IntegerType::less_than(10) < IntegerType::less_than(42));

        assert_eq!(IntegerType::less_than(42), IntegerType::less_than(42));

        assert!(IntegerType::greater_than(10) > IntegerType::equal(42));

        assert!(matches!(
            IntegerType::greater_than(42).partial_cmp(&IntegerType::equal(42)),
            None
        ));

        /*
        /// Expresses a constraint of `value <= 42`
        let joined = LessThan(42).join_integer(&Equal(42));

        /// See if that's compatible with `-10` - should be Some meet
        let result = joined.meet(&Integer(Equal(-10)));
        assert!(matches!( result, Some(_)));

        /// See if that's compatible with `42` - should be Some meet
        let result = joined.meet(&Integer(Equal(42)));
        assert!(matches!( result, Some(_)));

        /// See if that's compatible with `43` - should be None
        let result = joined.meet(&Integer(Equal(43)));
        assert!(matches!( result, None));
         */
    }
}
