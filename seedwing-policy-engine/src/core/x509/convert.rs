use crate::value::{Object, RuntimeValue};
use std::rc::Rc;
use std::str::from_utf8;
use std::sync::Arc;
use x509_parser::certificate::X509Certificate;
use x509_parser::der_parser::asn1_rs::{Any, BitString};
use x509_parser::der_parser::ber::{Class, Header};
use x509_parser::der_parser::Oid;
use x509_parser::extensions::{ExtendedKeyUsage, GeneralName, KeyUsage, X509Extension};
use x509_parser::prelude::{ParsedExtension, SubjectAlternativeName};
use x509_parser::x509::{
    AlgorithmIdentifier, AttributeTypeAndValue, RelativeDistinguishedName, SubjectPublicKeyInfo,
    X509Name,
};

impl From<&X509Certificate<'_>> for RuntimeValue {
    fn from(cert: &X509Certificate) -> Self {
        let mut obj = Object::new();

        obj.set("version", cert.version.0);
        obj.set("serial", cert.raw_serial_as_string());
        obj.set("subject", &cert.subject);
        obj.set("subject-pki", &cert.subject_pki);
        obj.set("issuer", &cert.issuer);
        obj.set("extensions", cert.extensions());

        obj.into()
    }
}

impl From<&[X509Certificate<'_>]> for RuntimeValue {
    fn from(bundle: &[X509Certificate<'_>]) -> Self {
        let mut seq: Vec<RuntimeValue> = Vec::with_capacity(bundle.len());

        for cert in bundle {
            seq.push(cert.into());
        }

        seq.into()
    }
}

impl From<&[&X509Certificate<'_>]> for RuntimeValue {
    fn from(bundle: &[&X509Certificate<'_>]) -> Self {
        let mut seq: Vec<RuntimeValue> = Vec::with_capacity(bundle.len());

        for cert in bundle {
            seq.push((*cert).into());
        }

        seq.into()
    }
}

impl From<&X509Name<'_>> for RuntimeValue {
    fn from(name: &X509Name<'_>) -> Self {
        let mut seq: Vec<RuntimeValue> = Vec::new();

        for rdn in name.iter() {
            seq.push(rdn.into())
        }

        seq.into()
    }
}

impl From<&RelativeDistinguishedName<'_>> for RuntimeValue {
    fn from(rdn: &RelativeDistinguishedName<'_>) -> Self {
        let mut seq: Vec<RuntimeValue> = Vec::new();

        for attr in rdn.iter() {
            seq.push(attr.into())
        }

        seq.into()
    }
}

impl From<&AttributeTypeAndValue<'_>> for RuntimeValue {
    fn from(attr: &AttributeTypeAndValue<'_>) -> Self {
        let mut obj = Object::new();

        obj.set("oid", attr.attr_type());
        if let Ok(data) = from_utf8(attr.attr_value().data) {
            obj.set("value", data)
        } else {
            obj.set("value", attr.attr_value().data)
        }

        obj.into()
    }
}

impl From<&Oid<'_>> for RuntimeValue {
    fn from(oid: &Oid<'_>) -> Self {
        let stringy = oid
            .as_bytes()
            .iter()
            .map(|e| format!("{e}"))
            .collect::<Vec<String>>()
            .join(".");

        stringy.into()
    }
}

impl From<&Any<'_>> for RuntimeValue {
    fn from(any: &Any<'_>) -> Self {
        let mut obj = Object::new();
        obj.set("header", (&any.header));
        obj.set("data", any.data);
        obj.into()
    }
}

impl From<&Header<'_>> for RuntimeValue {
    fn from(header: &Header<'_>) -> Self {
        let mut obj = Object::new();
        obj.set("class", &header.class());
        obj.into()
    }
}

impl From<&Class> for RuntimeValue {
    fn from(class: &Class) -> Self {
        let val = *class as u8;
        val.into()
    }
}

impl From<&[X509Extension<'_>]> for RuntimeValue {
    fn from(extensions: &[X509Extension<'_>]) -> Self {
        let mut seq: Vec<RuntimeValue> = Vec::new();

        for ext in extensions {
            if let Ok(ext) = ext.try_into() {
                seq.push(ext)
            }
        }

        seq.into()
    }
}

impl TryFrom<&X509Extension<'_>> for RuntimeValue {
    type Error = ();

    fn try_from(ext: &X509Extension<'_>) -> Result<Self, Self::Error> {
        ext.parsed_extension().try_into()
    }
}

impl TryFrom<&ParsedExtension<'_>> for RuntimeValue {
    type Error = ();

    fn try_from(ext: &ParsedExtension<'_>) -> Result<Self, Self::Error> {
        match ext {
            //ParsedExtension::UnsupportedExtension { .. } => {}
            //ParsedExtension::ParseError { .. } => {}
            //ParsedExtension::AuthorityKeyIdentifier(_) => {}
            //ParsedExtension::SubjectKeyIdentifier(_) => {}
            ParsedExtension::KeyUsage(key_usage) => {
                let mut obj = Object::new();
                obj.set("keyUsage", key_usage);
                Ok(obj.into())
            }
            //ParsedExtension::CertificatePolicies(_) => {}
            //ParsedExtension::PolicyMappings(_) => {}
            ParsedExtension::SubjectAlternativeName(name) => {
                let mut obj = Object::new();
                obj.set("subjectAlternativeName", name);
                Ok(obj.into())
            }
            //ParsedExtension::IssuerAlternativeName(_) => {}
            ParsedExtension::BasicConstraints(basic) => {
                let mut obj = Object::new();
                obj.set("CA", basic.ca);
                Ok(obj.into())
            }
            //ParsedExtension::NameConstraints(_) => {}
            //ParsedExtension::PolicyConstraints(_) => {}
            ParsedExtension::ExtendedKeyUsage(extended_key_usage) => {
                let mut obj = Object::new();
                obj.set("extendedKeyUsage", extended_key_usage);
                Ok(obj.into())
            }
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

impl From<&SubjectAlternativeName<'_>> for RuntimeValue {
    fn from(san: &SubjectAlternativeName<'_>) -> Self {
        let mut seq: Vec<RuntimeValue> = Vec::new();

        for name in &san.general_names {
            seq.push(name.into())
        }

        seq.into()
    }
}

impl From<&GeneralName<'_>> for RuntimeValue {
    fn from(name: &GeneralName<'_>) -> Self {
        match name {
            GeneralName::OtherName(_, _) => todo!(),
            GeneralName::RFC822Name(name) => {
                let mut obj = Object::new();
                obj.set("rfc822", *name);
                obj.into()
            }
            GeneralName::DNSName(name) => {
                let mut obj = Object::new();
                obj.set("DNS", *name);
                obj.into()
            }
            GeneralName::X400Address(_) => todo!(),
            GeneralName::DirectoryName(_) => todo!(),
            GeneralName::EDIPartyName(_) => todo!(),
            GeneralName::URI(_) => todo!(),
            GeneralName::IPAddress(_) => todo!(),
            GeneralName::RegisteredID(_) => todo!(),
        }
    }
}

impl From<&SubjectPublicKeyInfo<'_>> for RuntimeValue {
    fn from(value: &SubjectPublicKeyInfo<'_>) -> Self {
        let mut obj = Object::new();

        obj.set("public-key", &value.subject_public_key);
        obj.set("algorithm", &value.algorithm);
        obj.set("raw", value.raw);

        obj.into()
    }
}

impl From<&BitString<'_>> for RuntimeValue {
    fn from(value: &BitString<'_>) -> Self {
        RuntimeValue::Octets(value.data.to_vec())
    }
}

impl From<&AlgorithmIdentifier<'_>> for RuntimeValue {
    fn from(value: &AlgorithmIdentifier<'_>) -> Self {
        let mut obj = Object::new();

        obj.set("oid", &value.algorithm);
        obj.set(
            "parameters",
            value
                .parameters
                .as_ref()
                .map(|p| p.into())
                .unwrap_or(RuntimeValue::Null),
        );

        obj.into()
    }
}

impl From<&KeyUsage> for RuntimeValue {
    fn from(value: &KeyUsage) -> Self {
        let mut result = Vec::new();

        if value.digital_signature() {
            result.push(Arc::new("Digital Signature".into()));
        }
        if value.non_repudiation() {
            result.push(Arc::new("Non Repudiation".into()));
        }
        if value.key_encipherment() {
            result.push(Arc::new("Key Encipherment".into()));
        }
        if value.data_encipherment() {
            result.push(Arc::new("Data Encipherment".into()));
        }
        if value.key_agreement() {
            result.push(Arc::new("Key Agreement".into()));
        }
        if value.key_cert_sign() {
            result.push(Arc::new("Key Cert Sign".into()));
        }
        if value.crl_sign() {
            result.push(Arc::new("CRL Sign".into()));
        }
        if value.encipher_only() {
            result.push(Arc::new("Encipher Only".into()));
        }
        if value.decipher_only() {
            result.push(Arc::new("Decipher Only".into()));
        }

        RuntimeValue::List(result)
    }
}

impl From<&ExtendedKeyUsage<'_>> for RuntimeValue {
    fn from(value: &ExtendedKeyUsage<'_>) -> Self {
        let mut result = Object::new();

        result.set("any", value.any);
        result.set("serverAuth", value.server_auth);
        result.set("clientAuth", value.client_auth);
        result.set("codeSigning", value.code_signing);
        result.set("emailProtection", value.email_protection);
        result.set("timeStamping", value.time_stamping);
        result.set("ocspSigning", value.ocsp_signing);

        RuntimeValue::Object(result)
    }
}
