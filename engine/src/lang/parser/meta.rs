use crate::lang::{
    hir,
    parser::{
        literal::raw_string_literal,
        ty::{doc_comment, simple_type_name},
        Located, ParserError, ParserInput,
    },
};
use chumsky::prelude::*;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AttributeValue {
    /// A flag type value
    ///
    /// `#[example(flag)]` or `#[example(foo, "bar baz")]`
    Flag(Located<String>),
    /// A named value
    ///
    /// `#[example(bar=true)]` or `#[example(bar=true)]`
    Named {
        name: Located<String>,
        value: Located<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttributeDefn {
    pub name: Located<String>,
    pub values: HashMap<String, Option<Located<String>>>,
}

impl AttributeDefn {
    pub fn new(name: Located<String>, values: Vec<AttributeValue>) -> Self {
        Self {
            name,
            values: values
                .into_iter()
                .map(|v| match v {
                    AttributeValue::Flag(flag) => (flag.into_inner(), None),
                    AttributeValue::Named { name, value } => (name.into_inner(), Some(value)),
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Metadata {
    pub attributes: Vec<Located<AttributeDefn>>,
    pub documentation: String,
}

impl Metadata {
    pub fn documentation(&self) -> Option<String> {
        match self.documentation.is_empty() {
            true => None,
            false => Some(self.documentation.clone()),
        }
    }
}

impl From<Metadata> for hir::Metadata {
    fn from(value: Metadata) -> Self {
        let attributes = value
            .attributes
            .into_iter()
            .map(|attr| {
                let attr = attr.into_inner();
                (
                    attr.name.into_inner(),
                    hir::AttributeValues {
                        values: attr
                            .values
                            .into_iter()
                            .map(|(k, v)| (k, v.map(|v| v.into_inner())))
                            .collect::<HashMap<_, _>>(),
                    },
                )
            })
            .collect::<HashMap<_, _>>();

        Self {
            attributes,
            documentation: match value.documentation.is_empty() {
                true => None,
                false => Some(value.documentation),
            },
        }
    }
}

pub fn attribute_value(
) -> impl Parser<ParserInput, Located<AttributeValue>, Error = ParserError> + Clone {
    simple_type_name()
        .or(raw_string_literal())
        .padded()
        .then(
            just("=")
                .padded()
                .ignored()
                .then(simple_type_name().or(raw_string_literal()))
                .or_not(),
        )
        .map(|(name, value)| match value {
            Some(((), value)) => AttributeValue::Named { name, value },
            None => AttributeValue::Flag(name),
        })
        .map_with_span(Located::new)
}

pub fn attribute_definition(
) -> impl Parser<ParserInput, Located<AttributeDefn>, Error = ParserError> + Clone {
    simple_type_name()
        .padded()
        .then(
            attribute_value()
                .padded()
                .separated_by(just(",").padded())
                .allow_trailing()
                .delimited_by(just("("), just(")"))
                .padded()
                .map(|v| v.into_iter().map(|v| v.inner).collect::<Vec<_>>())
                .or_not(),
        )
        .delimited_by(just("#["), just("]"))
        .padded()
        .map(|(name, values)| AttributeDefn::new(name, values.unwrap_or_default()))
        .map_with_span(Located::new)
}

/// parse metadata, prepended to some element
pub fn metadata() -> impl Parser<ParserInput, Located<Metadata>, Error = ParserError> + Clone {
    #[derive(Clone, Debug)]
    enum Meta {
        Doc(String),
        Attributes(Located<AttributeDefn>),
    }

    impl FromIterator<Meta> for Metadata {
        fn from_iter<T: IntoIterator<Item = Meta>>(iter: T) -> Self {
            let mut attributes = Vec::new();
            let mut documentation = String::new();
            for i in iter {
                match i {
                    Meta::Attributes(attribute) => {
                        attributes.push(attribute);
                    }
                    Meta::Doc(partial_doc) => {
                        documentation.push_str(&partial_doc);
                    }
                }
            }
            Self {
                attributes,
                documentation,
            }
        }
    }

    choice::<_, ParserError>((
        attribute_definition().map(Meta::Attributes),
        // we require at least one, as otherwise we would forever try to read nothing
        doc_comment(1).map(Meta::Doc),
    ))
    .padded()
    .repeated()
    .collect::<Metadata>()
    .map_with_span(Located::new)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::lang::hir::AttributeValues;
    use crate::lang::parser::test::located;

    #[test]
    fn parse_attribute_plain() {
        let attr = attribute_definition().parse(r#"#[attr]"#).unwrap();
        assert_eq!(
            attr,
            Located::new(
                AttributeDefn::new(Located::new("attr".to_string(), 0..0usize.into()), vec![]),
                0..0usize.into()
            )
        );
    }

    #[test]
    fn parse_attribute_empty() {
        let attr = attribute_definition().parse(r#"#[attr()]"#).unwrap();
        assert_eq!(
            attr,
            Located::new(
                AttributeDefn::new(located("attr"), vec![]),
                0..0usize.into()
            )
        );
    }

    #[test]
    fn parse_attribute_flag() {
        let attr = attribute_definition().parse(r#"#[attr(foo)]"#).unwrap();
        assert_eq!(
            attr,
            Located::new(
                AttributeDefn::new(located("attr"), vec![AttributeValue::Flag(located("foo"))]),
                0..0usize.into()
            )
        );
    }

    #[test]
    fn parse_attribute_flags() {
        let attr = attribute_definition()
            .parse(r#"#[attr(foo, bar)]"#)
            .unwrap();
        assert_eq!(
            attr,
            Located::new(
                AttributeDefn::new(
                    located("attr"),
                    vec![
                        AttributeValue::Flag(located("foo")),
                        AttributeValue::Flag(located("bar"))
                    ]
                ),
                0..0usize.into()
            )
        );
    }

    #[test]
    fn parse_attribute_flags_trailing() {
        let attr = attribute_definition()
            .parse(r#"#[attr(foo, bar, )]"#)
            .unwrap();
        assert_eq!(
            attr,
            Located::new(
                AttributeDefn::new(
                    located("attr"),
                    vec![
                        AttributeValue::Flag(located("foo")),
                        AttributeValue::Flag(located("bar"))
                    ]
                ),
                0..0usize.into()
            )
        );
    }

    #[test]
    fn parse_attribute_field() {
        let attr = attribute_definition()
            .parse(r#"#[attr(foo = true)]"#)
            .unwrap();
        assert_eq!(
            attr,
            Located::new(
                AttributeDefn::new(
                    located("attr"),
                    vec![AttributeValue::Named {
                        name: located("foo"),
                        value: located("true")
                    },]
                ),
                0..0usize.into()
            )
        );
    }

    #[test]
    fn parse_attribute_mixed() {
        let attr = attribute_definition()
            .parse(r#"#[ attr ( foo = true, flag ) ]"#)
            .unwrap();
        assert_eq!(
            attr,
            Located::new(
                AttributeDefn::new(
                    located("attr"),
                    vec![
                        AttributeValue::Named {
                            name: located("foo"),
                            value: located("true")
                        },
                        AttributeValue::Flag(located("flag"))
                    ]
                ),
                0..0usize.into()
            )
        );
    }

    #[test]
    fn parse_attribute_string() {
        let attr = attribute_definition()
            .parse(r#"#[ attr ( "foo bar") ]"#)
            .unwrap();
        assert_eq!(
            attr,
            Located::new(
                AttributeDefn::new(
                    located("attr"),
                    vec![AttributeValue::Flag(located("foo bar")),]
                ),
                0..0usize.into()
            )
        );
    }

    #[test]
    fn parse_attribute_mixed_strings() {
        let attr = attribute_definition()
            .parse(r#"#[ attr ( foo = true, "flag", bar   = "baz" ) ]"#)
            .unwrap();
        assert_eq!(
            attr,
            Located::new(
                AttributeDefn::new(
                    located("attr"),
                    vec![
                        AttributeValue::Named {
                            name: located("foo"),
                            value: located("true")
                        },
                        AttributeValue::Flag(located("flag")),
                        AttributeValue::Named {
                            name: located("bar"),
                            value: located("baz")
                        },
                    ]
                ),
                0..0usize.into()
            )
        );
    }

    #[test]
    fn convert_metadata() {
        let meta = Metadata {
            documentation: "foo bar".to_string(),
            attributes: vec![located(AttributeDefn {
                name: located("attr1"),
                values: {
                    let mut v = HashMap::new();
                    v.insert("f1".to_string(), None);
                    v.insert("f2".to_string(), Some(located("v2".to_string())));
                    v
                },
            })],
        };

        // now convert

        let meta: hir::Metadata = meta.into();

        // check

        assert_eq!(meta.documentation, Some("foo bar".to_string()));
        assert_eq!(meta.attributes, {
            let mut m = HashMap::new();
            m.insert(
                "attr1".to_string(),
                AttributeValues {
                    values: [
                        ("f1".to_string(), None),
                        ("f2".to_string(), Some("v2".to_string())),
                    ]
                    .into(),
                },
            );
            m
        });
    }
}
