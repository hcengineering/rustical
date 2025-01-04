use quick_xml::events::{BytesEnd, BytesStart, BytesText, Event};
use quick_xml::name::{Namespace, QName};
use std::collections::HashMap;
use std::num::{ParseFloatError, ParseIntError};
use std::{convert::Infallible, io::BufRead};
use thiserror::Error;

use crate::{XmlDeError, XmlDeserialize, XmlSerialize};

#[derive(Debug, Error)]
pub enum ParseValueError {
    #[error(transparent)]
    Infallible(#[from] Infallible),
    #[error(transparent)]
    ParseIntError(#[from] ParseIntError),
    #[error(transparent)]
    ParseFloatError(#[from] ParseFloatError),
}

pub trait Value: Sized {
    fn serialize(&self) -> String;
    fn deserialize(val: &str) -> Result<Self, XmlDeError>;
}

macro_rules! impl_value_parse {
    ($t:ty) => {
        impl Value for $t {
            fn serialize(&self) -> String {
                self.to_string()
            }

            fn deserialize(val: &str) -> Result<Self, XmlDeError> {
                val.parse()
                    .map_err(ParseValueError::from)
                    .map_err(XmlDeError::from)
            }
        }
    };
}

impl_value_parse!(String);
impl_value_parse!(i8);
impl_value_parse!(u8);
impl_value_parse!(i16);
impl_value_parse!(u16);
impl_value_parse!(f32);
impl_value_parse!(i32);
impl_value_parse!(u32);
impl_value_parse!(f64);
impl_value_parse!(i64);
impl_value_parse!(u64);
impl_value_parse!(isize);
impl_value_parse!(usize);

impl Value for &str {
    fn serialize(&self) -> String {
        self.to_string()
    }

    fn deserialize(_val: &str) -> Result<Self, XmlDeError> {
        Err(XmlDeError::Other("TODO: Handle this error".to_owned()))
    }
}

impl<T: Value> XmlDeserialize for T {
    fn deserialize<R: BufRead>(
        reader: &mut quick_xml::NsReader<R>,
        _start: &BytesStart,
        empty: bool,
    ) -> Result<Self, XmlDeError> {
        let mut string = String::new();

        if !empty {
            let mut buf = Vec::new();
            loop {
                match reader.read_event_into(&mut buf)? {
                    Event::Text(text) => {
                        if !string.is_empty() {
                            // Content already written
                            return Err(XmlDeError::UnsupportedEvent("content already written"));
                        }
                        string = String::from_utf8_lossy(text.as_ref()).to_string();
                    }
                    Event::End(_) => break,
                    Event::Eof => return Err(XmlDeError::Eof),
                    _ => return Err(XmlDeError::UnsupportedEvent("todo")),
                };
            }
        }

        Value::deserialize(&string)
    }
}

impl<T: Value> XmlSerialize for T {
    fn serialize<W: std::io::Write>(
        &self,
        ns: Option<Namespace>,
        tag: Option<&[u8]>,
        namespaces: &HashMap<Namespace, &[u8]>,
        writer: &mut quick_xml::Writer<W>,
    ) -> std::io::Result<()> {
        let prefix = ns
            .map(|ns| namespaces.get(&ns))
            .unwrap_or(None)
            .map(|prefix| {
                if !prefix.is_empty() {
                    [*prefix, b":"].concat()
                } else {
                    Vec::new()
                }
            });
        let has_prefix = prefix.is_some();
        let tagname = tag.map(|tag| [&prefix.unwrap_or_default(), tag].concat());
        let qname = tagname.as_ref().map(|tagname| QName(tagname));
        if let Some(qname) = &qname {
            let mut bytes_start = BytesStart::from(qname.to_owned());
            if !has_prefix {
                if let Some(ns) = &ns {
                    bytes_start.push_attribute((b"xmlns".as_ref(), ns.as_ref()));
                }
            }
            writer.write_event(Event::Start(bytes_start))?;
        }
        writer.write_event(Event::Text(BytesText::new(&self.serialize())))?;
        if let Some(qname) = &qname {
            writer.write_event(Event::End(BytesEnd::from(qname.to_owned())))?;
        }
        Ok(())
    }

    #[allow(refining_impl_trait)]
    fn attributes<'a>(&self) -> Option<Vec<quick_xml::events::attributes::Attribute<'a>>> {
        None
    }
}
