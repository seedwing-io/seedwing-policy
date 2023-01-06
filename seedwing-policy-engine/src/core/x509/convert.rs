use crate::value::{InputValue, Object};
use std::str::from_utf8;
use x509_parser::certificate::X509Certificate;
use x509_parser::der_parser::asn1_rs::Any;
use x509_parser::der_parser::ber::{Class, Header};
use x509_parser::der_parser::Oid;
use x509_parser::extensions::{GeneralName, X509Extension};
use x509_parser::prelude::{ParsedExtension, SubjectAlternativeName};
use x509_parser::x509::{AttributeTypeAndValue, RelativeDistinguishedName, X509Name};

impl From<&X509Certificate<'_>> for InputValue {
    fn from(cert: &X509Certificate) -> Self {
        let mut obj = Object::new();

        obj.set("version".into(), cert.version.0.into());
        obj.set("issuer".into(), (&cert.issuer).into());
        obj.set("extensions".into(), cert.extensions().into());

        obj.into()
    }
}

impl From<&X509Name<'_>> for InputValue {
    fn from(name: &X509Name<'_>) -> Self {
        let mut seq: Vec<InputValue> = Vec::new();

        for rdn in name.iter() {
            seq.push(rdn.into())
        }

        seq.into()
    }
}

impl From<&RelativeDistinguishedName<'_>> for InputValue {
    fn from(rdn: &RelativeDistinguishedName<'_>) -> Self {
        let mut seq: Vec<InputValue> = Vec::new();

        for attr in rdn.iter() {
            seq.push(attr.into())
        }

        seq.into()
    }
}

impl From<&AttributeTypeAndValue<'_>> for InputValue {
    fn from(attr: &AttributeTypeAndValue<'_>) -> Self {
        let mut obj = Object::new();

        obj.set("oid".into(), attr.attr_type().into());
        if let Ok(data) = from_utf8(attr.attr_value().data) {
            obj.set("value".into(), data.into())
        } else {
            obj.set("value".into(), attr.attr_value().data.into())
        }

        obj.into()
    }
}

impl From<&Oid<'_>> for InputValue {
    fn from(oid: &Oid<'_>) -> Self {
        let stringy = oid
            .as_bytes()
            .iter()
            .map(|e| format!("{}", e))
            .collect::<Vec<String>>()
            .join(".");

        stringy.into()
    }
}

impl From<&Any<'_>> for InputValue {
    fn from(any: &Any<'_>) -> Self {
        let mut obj = Object::new();
        obj.set("header".into(), (&any.header).into());
        obj.set("data".into(), any.data.into());
        obj.into()
    }
}

impl From<&Header<'_>> for InputValue {
    fn from(header: &Header<'_>) -> Self {
        let mut obj = Object::new();
        obj.set("class".into(), (&header.class()).into());
        obj.into()
    }
}

impl From<&Class> for InputValue {
    fn from(class: &Class) -> Self {
        let val = *class as u8;
        val.into()
    }
}

impl From<&[X509Extension<'_>]> for InputValue {
    fn from(extensions: &[X509Extension<'_>]) -> Self {
        let mut seq: Vec<InputValue> = Vec::new();

        for ext in extensions {
            if let Ok(ext) = ext.try_into() {
                seq.push(ext)
            }
        }

        seq.into()
    }
}

impl TryFrom<&X509Extension<'_>> for InputValue {
    type Error = ();

    fn try_from(ext: &X509Extension<'_>) -> Result<Self, Self::Error> {
        ext.parsed_extension().try_into()
    }
}

impl TryFrom<&ParsedExtension<'_>> for InputValue {
    type Error = ();

    fn try_from(ext: &ParsedExtension<'_>) -> Result<Self, Self::Error> {
        match ext {
            //ParsedExtension::UnsupportedExtension { .. } => {}
            //ParsedExtension::ParseError { .. } => {}
            //ParsedExtension::AuthorityKeyIdentifier(_) => {}
            //ParsedExtension::SubjectKeyIdentifier(_) => {}
            //ParsedExtension::KeyUsage(_) => {}
            //ParsedExtension::CertificatePolicies(_) => {}
            //ParsedExtension::PolicyMappings(_) => {}
            ParsedExtension::SubjectAlternativeName(name) => {
                let mut obj = Object::new();
                obj.set("subjectAlternativeName".into(), name.into());
                Ok(obj.into())
            }
            //ParsedExtension::IssuerAlternativeName(_) => {}
            //ParsedExtension::BasicConstraints(_) => {}
            //ParsedExtension::NameConstraints(_) => {}
            //ParsedExtension::PolicyConstraints(_) => {}
            //ParsedExtension::ExtendedKeyUsage(_) => {}
            //ParsedExtension::CRLDistributionPoints(_) => {}
            //ParsedExtension::InhibitAnyPolicy(_) => {}
            //ParsedExtension::AuthorityInfoAccess(_) => {}
            //ParsedExtension::NSCertType(_) => {}
            //ParsedExtension::NsCertComment(_) => {}
            //ParsedExtension::CRLNumber(_) => {}
            //ParsedExtension::ReasonCode(_) => {}
            //ParsedExtension::InvalidityDate(_) => {}
            //ParsedExtension::SCT(_) => {}
            //ParsedExtension::Unparsed => {}
            _ => Err(()),
        }
    }
}

impl From<&SubjectAlternativeName<'_>> for InputValue {
    fn from(san: &SubjectAlternativeName<'_>) -> Self {
        let mut seq: Vec<InputValue> = Vec::new();

        for name in &san.general_names {
            seq.push(name.into())
        }

        seq.into()
    }
}

impl From<&GeneralName<'_>> for InputValue {
    fn from(name: &GeneralName<'_>) -> Self {
        match name {
            GeneralName::OtherName(_, _) => todo!(),
            GeneralName::RFC822Name(name) => {
                let mut obj = Object::new();
                obj.set("rfc822".into(), (*name).into());
                obj.into()
            }
            GeneralName::DNSName(_) => todo!(),
            GeneralName::X400Address(_) => todo!(),
            GeneralName::DirectoryName(_) => todo!(),
            GeneralName::EDIPartyName(_) => todo!(),
            GeneralName::URI(_) => todo!(),
            GeneralName::IPAddress(_) => todo!(),
            GeneralName::RegisteredID(_) => todo!(),
        }
    }
}
